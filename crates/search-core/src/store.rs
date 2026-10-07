use crate::types::*;
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use std::path::Path;

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}
pub struct Store {
    pub db: Connection,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?
        }
        let db = Connection::open(path)?;
        db.busy_timeout(std::time::Duration::from_secs(10))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL;
        CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS roots(id INTEGER PRIMARY KEY,path TEXT UNIQUE NOT NULL,online INTEGER NOT NULL DEFAULT 1,exclusions TEXT NOT NULL DEFAULT '[]');
        CREATE TABLE IF NOT EXISTS files(id INTEGER PRIMARY KEY,root_id INTEGER REFERENCES roots(id) ON DELETE CASCADE,path TEXT UNIQUE NOT NULL,name TEXT NOT NULL,extension TEXT,kind TEXT,size INTEGER,modified INTEGER,created INTEGER,state TEXT NOT NULL DEFAULT 'queued',semantic INTEGER DEFAULT 0,error TEXT,online INTEGER DEFAULT 1,content_hash TEXT,seen INTEGER DEFAULT 0,stable_id TEXT,last_indexed INTEGER);
        CREATE INDEX IF NOT EXISTS files_root ON files(root_id); CREATE INDEX IF NOT EXISTS files_name ON files(name COLLATE NOCASE); CREATE INDEX IF NOT EXISTS files_hash ON files(content_hash);
        CREATE TABLE IF NOT EXISTS chunks(id INTEGER PRIMARY KEY AUTOINCREMENT,file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,seq INTEGER,modality TEXT,text TEXT,heading TEXT,page INTEGER,line_start INTEGER,line_end INTEGER,start REAL,end REAL,hash TEXT,vector BLOB,namespace TEXT);
        CREATE INDEX IF NOT EXISTS chunks_file ON chunks(file_id);
        CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(name,path,content='files',content_rowid='id',tokenize='unicode61');
        CREATE TRIGGER IF NOT EXISTS files_ai AFTER INSERT ON files BEGIN INSERT INTO files_fts(rowid,name,path) VALUES(new.id,new.name,new.path); END;
        CREATE TRIGGER IF NOT EXISTS files_ad AFTER DELETE ON files BEGIN INSERT INTO files_fts(files_fts,rowid,name,path) VALUES('delete',old.id,old.name,old.path); END;
        CREATE TRIGGER IF NOT EXISTS files_au AFTER UPDATE OF name,path ON files BEGIN INSERT INTO files_fts(files_fts,rowid,name,path) VALUES('delete',old.id,old.name,old.path); INSERT INTO files_fts(rowid,name,path) VALUES(new.id,new.name,new.path); END;
        CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(text,heading,content='chunks',content_rowid='id',tokenize='unicode61');
        CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN INSERT INTO chunks_fts(rowid,text,heading) VALUES(new.id,new.text,new.heading); END;
        CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN INSERT INTO chunks_fts(chunks_fts,rowid,text,heading) VALUES('delete',old.id,old.text,old.heading); END;
        CREATE TABLE IF NOT EXISTS jobs(file_id INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,state TEXT NOT NULL DEFAULT 'queued');
        CREATE TABLE IF NOT EXISTS activity(id INTEGER PRIMARY KEY,time INTEGER,kind TEXT,file_id INTEGER,path TEXT,message TEXT);
        CREATE TABLE IF NOT EXISTS benchmark(fingerprint TEXT PRIMARY KEY,result TEXT,measured INTEGER);
        PRAGMA user_version=1;")?;
        db.execute("UPDATE jobs SET state='queued' WHERE state='active'", [])?;
        let columns: Vec<String> = db
            .prepare("PRAGMA table_info(files)")?
            .query_map([], |row| row.get(1))?
            .collect::<rusqlite::Result<_>>()?;
        if !columns.iter().any(|name| name == "mtime_ns") {
            db.execute(
                "ALTER TABLE files ADD COLUMN mtime_ns INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        Ok(Self { db })
    }
    pub fn settings(&self) -> Result<Settings> {
        let value = self
            .db
            .query_row("SELECT value FROM settings WHERE key='app'", [], |r| {
                r.get::<_, String>(0)
            });
        match value {
            Ok(v) => Ok(serde_json::from_str(&v)?),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Settings::default()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn save_settings(&self, s: &Settings) -> Result<()> {
        s.validate()?;
        self.db.execute(
            "INSERT OR REPLACE INTO settings VALUES('app',?1)",
            [serde_json::to_string(s)?],
        )?;
        Ok(())
    }
    pub fn save_configuration(&self, settings: &Settings, clear_semantics: bool) -> Result<()> {
        let transaction = self.db.unchecked_transaction()?;
        self.save_settings(settings)?;
        if clear_semantics {
            transaction.execute("UPDATE chunks SET vector=NULL", [])?;
            transaction.execute("UPDATE files SET semantic=0,state='queued'", [])?;
            transaction.execute(
                "INSERT OR REPLACE INTO jobs(file_id) SELECT id FROM files",
                [],
            )?;
        }
        for (kind, enabled) in [
            ("text", settings.text),
            ("code", settings.code),
            ("document", settings.documents),
            ("pdf", settings.documents),
            ("image", settings.images),
            ("audio", settings.audio),
            ("video", settings.video),
        ] {
            if enabled {
                transaction.execute("INSERT OR IGNORE INTO jobs(file_id) SELECT id FROM files WHERE kind=?1 AND state='metadata_only'", [kind])?;
            }
        }
        transaction.commit()?;
        Ok(())
    }
    pub fn roots(&self) -> Result<Vec<Root>> {
        let mut q=self.db.prepare("SELECT r.id,r.path,r.online,r.exclusions,count(f.id),coalesce(sum(f.semantic),0) FROM roots r LEFT JOIN files f ON f.root_id=r.id GROUP BY r.id ORDER BY r.id")?;
        Ok(q.query_map([], |r| {
            let s: String = r.get(3)?;
            Ok(Root {
                id: r.get(0)?,
                path: r.get(1)?,
                online: r.get(2)?,
                files: r.get(4)?,
                indexed: r.get(5)?,
                exclusions: serde_json::from_str(&s).unwrap_or_default(),
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
    }
    pub fn add_root(&self, path: &str) -> Result<i64> {
        self.db
            .execute("INSERT INTO roots(path) VALUES(?1)", [path])?;
        Ok(self.db.last_insert_rowid())
    }
    pub fn file(&self, id: i64) -> Result<FileRecord> {
        Ok(self.db.query_row("SELECT id,root_id,path,name,extension,kind,size,modified,created,state,semantic,error,online FROM files WHERE id=?1",[id],Self::read_file)?)
    }
    pub fn read_file(r: &rusqlite::Row<'_>) -> rusqlite::Result<FileRecord> {
        Ok(FileRecord {
            id: r.get(0)?,
            root_id: r.get(1)?,
            path: r.get(2)?,
            name: r.get(3)?,
            extension: r.get(4)?,
            kind: r.get(5)?,
            size: r.get(6)?,
            modified: r.get(7)?,
            created: r.get(8)?,
            state: r.get(9)?,
            semantic: r.get(10)?,
            error: r.get(11)?,
            online: r.get(12)?,
        })
    }
    pub fn chunks(&self, id: i64) -> Result<Vec<Chunk>> {
        let mut q=self.db.prepare("SELECT id,modality,text,heading,page,line_start,line_end,start,end,hash FROM chunks WHERE file_id=?1 ORDER BY seq LIMIT 500")?;
        Ok(q.query_map([id], Self::read_chunk)?
            .collect::<rusqlite::Result<_>>()?)
    }
    pub fn chunk(&self, id: i64) -> Result<Chunk> {
        Ok(self.db.query_row("SELECT id,modality,text,heading,page,line_start,line_end,start,end,hash FROM chunks WHERE id=?1",[id],Self::read_chunk)?)
    }
    fn read_chunk(r: &rusqlite::Row<'_>) -> rusqlite::Result<Chunk> {
        Ok(Chunk {
            id: r.get(0)?,
            modality: r.get(1)?,
            text: r.get(2)?,
            heading: r.get(3)?,
            page: r.get(4)?,
            line_start: r.get(5)?,
            line_end: r.get(6)?,
            start: r.get(7)?,
            end: r.get(8)?,
            hash: r.get(9)?,
            ..Default::default()
        })
    }
    pub fn activity(&self) -> Result<Vec<Activity>> {
        let mut q = self.db.prepare(
            "SELECT id,file_id,time,kind,path,message FROM activity ORDER BY id DESC LIMIT 500",
        )?;
        Ok(q.query_map([], |r| {
            Ok(Activity {
                id: r.get(0)?,
                file_id: r.get(1)?,
                time: r.get(2)?,
                kind: r.get(3)?,
                path: r.get(4)?,
                message: r.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?)
    }
    pub fn log(&self, kind: &str, file: Option<i64>, path: &str, message: &str) -> Result<()> {
        self.db.execute(
            "INSERT INTO activity(time,kind,file_id,path,message) VALUES(?1,?2,?3,?4,?5)",
            params![now(), kind, file, path, message],
        )?;
        self.db.execute(
            "DELETE FROM activity WHERE id < (SELECT coalesce(max(id),0)-2000 FROM activity)",
            [],
        )?;
        Ok(())
    }
    pub fn health(&self) -> Result<serde_json::Value> {
        let integrity: String = self
            .db
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
        let foreign: i64 =
            self.db
                .query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
                    r.get(0)
                })?;
        Ok(serde_json::json!({"integrity":integrity,"foreign_key_errors":foreign,"schema":1}))
    }
    pub fn commit_chunks(
        &mut self,
        id: i64,
        chunks: &[(Chunk, Option<Vec<f32>>)],
        hash: &str,
        partial: Option<&str>,
    ) -> Result<Vec<i64>> {
        let tx = self.db.transaction()?;
        let mut q = tx.prepare("SELECT id FROM chunks WHERE file_id=?1")?;
        let old = q
            .query_map([id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<i64>>>()?;
        drop(q);
        tx.execute("DELETE FROM chunks WHERE file_id=?1", [id])?;
        let mut semantic = false;
        for (seq, (c, v)) in chunks.iter().enumerate() {
            let bytes = v
                .as_ref()
                .map(|v| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<_>>());
            semantic |= v.is_some();
            tx.execute("INSERT INTO chunks(file_id,seq,modality,text,heading,page,line_start,line_end,start,end,hash,vector,namespace) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![id,seq as i64,c.modality,c.text,c.heading,c.page,c.line_start,c.line_end,c.start,c.end,c.hash,bytes,if c.modality=="code"{"code"}else{"general"}])?;
        }
        let state = if partial.is_some() {
            "partial"
        } else if semantic {
            "indexed"
        } else if chunks.is_empty() {
            "metadata_only"
        } else {
            "partial"
        };
        tx.execute("UPDATE files SET content_hash=?2,state=?3,semantic=?4,error=?5,last_indexed=?6 WHERE id=?1",params![id,hash,state,semantic,partial,now()])?;
        tx.execute("DELETE FROM jobs WHERE file_id=?1", [id])?;
        tx.commit().context("Atomic content commit")?;
        Ok(old)
    }
}
