use crate::{
    extract,
    inference::Inference,
    query,
    store::{Store, now},
    types::*,
    vectors::Vectors,
};
use anyhow::{Context, Result, ensure};
use notify::{RecursiveMode, Watcher};
use rusqlite::params;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant, UNIX_EPOCH},
};

pub type EventSink = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;
pub struct Engine {
    pub store: Mutex<Store>,
    pub vectors: Mutex<Vectors>,
    pub inference: Inference,
    pub data: PathBuf,
    pub resources: PathBuf,
    pub status: Mutex<IndexStatus>,
    pub paused: AtomicBool,
    pub stopped: AtomicBool,
    pub running: AtomicBool,
    pub latest_query: AtomicU64,
    pub scan: Mutex<Vec<i64>>,
    pub events: EventSink,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
    work: Mutex<()>,
    configuration: RwLock<()>,
    last_emit: Mutex<Instant>,
    changes: Mutex<HashMap<PathBuf, Instant>>,
}
impl Engine {
    pub fn open(data: PathBuf, resources: PathBuf, events: EventSink) -> Result<Arc<Self>> {
        std::fs::create_dir_all(data.join("cache"))?;
        let store = Store::open(&data.join("data/metadata.sqlite"))?;
        let settings = store.settings()?;
        let mut vectors = Vectors::new(settings.dimensions)?;
        let vector_signature: String = store.db.query_row(
            "SELECT count(*) || ':' || coalesce(max(id),0) FROM chunks WHERE vector IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        let expected_signature = format!("{}:{vector_signature}", settings.fingerprint());
        let checkpoint = store
            .db
            .query_row(
                "SELECT value FROM settings WHERE key='vector_checkpoint'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok();
        let loaded = checkpoint.as_deref() == Some(&expected_signature)
            && vectors.load(&data.join("data/vectors")).is_ok();
        if !loaded {
            vectors = Vectors::new(settings.dimensions)?;
            let mut q = store
                .db
                .prepare("SELECT id,namespace,vector FROM chunks WHERE vector IS NOT NULL")?;
            let rows = q.query_map([], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                ))
            })?;
            for row in rows {
                let (id, ns, bytes) = row?;
                let v: Vec<f32> = bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|b| f32::from_le_bytes(*b))
                    .collect();
                if v.len() == settings.dimensions {
                    vectors.add(id as u64, &ns, &v)?
                }
            }
        }
        let data = PathBuf::from(display_path(&std::fs::canonicalize(data)?));
        let resources = PathBuf::from(display_path(&std::fs::canonicalize(resources)?));
        let inference = Inference::start(resources.clone(), data.join("cache"), settings);
        let engine = Arc::new(Self {
            store: Mutex::new(store),
            vectors: Mutex::new(vectors),
            inference,
            data,
            resources,
            status: Mutex::new(IndexStatus {
                stage: "ready".into(),
                watcher: "starting".into(),
                ..Default::default()
            }),
            paused: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            running: AtomicBool::new(false),
            latest_query: AtomicU64::new(0),
            scan: Mutex::new(Vec::new()),
            events,
            watcher: Mutex::new(None),
            work: Mutex::new(()),
            configuration: RwLock::new(()),
            last_emit: Mutex::new(Instant::now() - Duration::from_secs(1)),
            changes: Mutex::new(HashMap::new()),
        });
        engine.watch()?;
        // Reconcile changes made while the app was closed. Unchanged readable
        // NTFS journals skip enumeration; other volumes use metadata-only scans.
        for root in engine.roots()? {
            let journal = crate::platform::journal_checkpoint(Path::new(&root.path));
            let saved = engine
                .store
                .lock()
                .unwrap()
                .db
                .query_row(
                    "SELECT value FROM settings WHERE key=?1",
                    [format!("journal:{}", root.id)],
                    |r| r.get::<_, String>(0),
                )
                .ok();
            if journal.is_none() || journal != saved {
                engine.scan.lock().unwrap().push(root.id);
            }
        }
        let weak = Arc::downgrade(&engine);
        thread::spawn(move || {
            let mut was_ready = false;
            let mut previous_health = serde_json::Value::Null;
            let mut last_reconcile = Instant::now();
            loop {
                thread::sleep(Duration::from_millis(250));
                let Some(e) = weak.upgrade() else { break };
                let ready = e.inference.ready();
                let health = e.inference.health.lock().unwrap().clone();
                if health != previous_health {
                    previous_health = health.clone();
                    (e.events)("inference-state", health);
                    e.emit_status();
                }
                if ready && !was_ready {
                    let s = e.store.lock().unwrap();
                    let _ = s.db.execute("INSERT OR REPLACE INTO jobs(file_id) SELECT id FROM files WHERE state='partial' AND error LIKE 'Semantic model unavailable%'", []);
                }
                was_ready = ready;
                let mut paths = Vec::new();
                e.changes.lock().unwrap().retain(|path, time| {
                    if time.elapsed() > Duration::from_millis(500) {
                        paths.push(path.clone());
                        false
                    } else {
                        true
                    }
                });
                paths.sort_by_key(|path| !path.exists());
                for path in paths {
                    let _ = e.changed(&path);
                }
                if e.paused.load(Ordering::Relaxed) || e.stopped.load(Ordering::Relaxed) {
                    continue;
                }
                if last_reconcile.elapsed() > Duration::from_secs(300) {
                    if let Ok(roots) = e.roots() {
                        let mut scan = e.scan.lock().unwrap();
                        for root in roots {
                            if !scan.contains(&root.id) {
                                scan.push(root.id)
                            }
                        }
                    }
                    last_reconcile = Instant::now();
                }
                let scans = std::mem::take(&mut *e.scan.lock().unwrap());
                if !scans.is_empty() {
                    e.running.store(true, Ordering::Relaxed);
                    for id in scans {
                        if let Err(err) = e.discover(id) {
                            let _ =
                                e.store
                                    .lock()
                                    .unwrap()
                                    .log("error", None, "", &err.to_string());
                        }
                    }
                }
                let job = {
                    let s = e.store.lock().unwrap();
                    let media_paused = s.settings().is_ok_and(|settings| settings.pause_on_battery)
                        && crate::platform::power_constrained();
                    s.db.query_row("SELECT f.id FROM jobs j JOIN files f ON f.id=j.file_id WHERE j.state='queued' AND f.online=1 AND (?1=0 OR f.kind NOT IN ('image','audio','video')) ORDER BY CASE f.kind WHEN 'text' THEN 0 WHEN 'code' THEN 1 WHEN 'document' THEN 2 WHEN 'pdf' THEN 2 WHEN 'image' THEN 3 WHEN 'audio' THEN 4 ELSE 5 END,f.id LIMIT 1",[media_paused],|r|r.get::<_,i64>(0)).ok()
                };
                if let Some(id) = job {
                    e.running.store(true, Ordering::Relaxed);
                    if let Err(err) = e.index_file(id) {
                        let s = e.store.lock().unwrap();
                        let _ = s.db.execute(
                            "UPDATE files SET state='error',error=?2 WHERE id=?1",
                            params![id, err.to_string()],
                        );
                        let _ = s.db.execute("DELETE FROM jobs WHERE file_id=?1", [id]);
                        let path = s.file(id).map(|f| f.path).unwrap_or_default();
                        let _ = s.log("error", Some(id), &path, &err.to_string());
                    }
                    e.emit_status();
                } else if e.running.swap(false, Ordering::Relaxed) {
                    let _ = e.save_vectors();
                    let pending = e
                        .store
                        .lock()
                        .unwrap()
                        .db
                        .query_row("SELECT count(*) FROM jobs", [], |r| r.get::<_, i64>(0))
                        .unwrap_or(0);
                    e.status.lock().unwrap().stage = if pending == 0 {
                        "ready"
                    } else {
                        "battery_paused"
                    }
                    .into();
                    e.emit_status();
                    if pending == 0 {
                        (e.events)("index-complete", serde_json::json!({"time":now()}));
                    }
                }
            }
        });
        Ok(engine)
    }
    pub fn settings(&self) -> Result<Settings> {
        self.store.lock().unwrap().settings()
    }
    fn save_vectors(&self) -> Result<()> {
        // DB/vector generations are checked together. A crash triggers recovery
        // from SQLite's committed vectors, never a silently stale ANN snapshot.
        let s = self.store.lock().unwrap();
        let signature: String = s.db.query_row(
            "SELECT count(*) || ':' || coalesce(max(id),0) FROM chunks WHERE vector IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        let fingerprint = s.settings()?.fingerprint();
        self.vectors
            .lock()
            .unwrap()
            .save(&self.data.join("data/vectors"))?;
        s.db.execute(
            "INSERT OR REPLACE INTO settings VALUES('vector_checkpoint',?1)",
            [format!("{fingerprint}:{signature}")],
        )?;
        Ok(())
    }
    pub fn move_index(&self, destination: &Path) -> Result<PathBuf> {
        ensure!(
            self.stopped.load(Ordering::Relaxed),
            "Stop indexing before moving storage"
        );
        let _work = self.work.lock().unwrap();
        let destination = std::fs::canonicalize(destination)?.join("Smarti Local Search Index");
        ensure!(
            !destination.exists(),
            "The destination already contains an index directory"
        );
        ensure!(
            !destination.starts_with(&self.data),
            "Choose a location outside the current index"
        );
        std::fs::create_dir_all(destination.join("data"))?;
        let s = self.store.lock().unwrap();
        s.db.execute(
            "VACUUM INTO ?1",
            [display_path(&destination.join("data/metadata.sqlite"))],
        )?;
        self.vectors
            .lock()
            .unwrap()
            .save(&destination.join("data/vectors"))?;
        let copy = Store::open(&destination.join("data/metadata.sqlite"))?;
        let mut settings = copy.settings()?;
        settings.index_path = display_path(&destination);
        copy.save_settings(&settings)?;
        ensure!(
            copy.health()?["integrity"] == "ok",
            "Copied index failed validation"
        );
        Ok(destination)
    }
    pub fn roots(&self) -> Result<Vec<Root>> {
        self.store.lock().unwrap().roots()
    }
    pub fn add_root(self: &Arc<Self>, path: &Path) -> Result<i64> {
        let path = PathBuf::from(display_path(
            &std::fs::canonicalize(path).context("Folder is not accessible")?,
        ));
        ensure!(path.is_dir(), "Select a folder or drive");
        let settings = self.settings()?;
        ensure!(
            settings.network_drives
                || !path.to_string_lossy().starts_with("\\\\")
                || path.to_string_lossy().starts_with("\\\\?\\"),
            "Enable network drives in Settings first"
        );
        let text = display_path(&path);
        for r in self.roots()? {
            let existing = PathBuf::from(&r.path);
            ensure!(
                !path.starts_with(&existing) && !existing.starts_with(&path),
                "This location overlaps an indexed folder"
            )
        }
        let id = self.store.lock().unwrap().add_root(&text)?;
        self.watch()?;
        Ok(id)
    }
    pub fn remove_root(self: &Arc<Self>, id: i64) -> Result<()> {
        ensure!(
            !self.running.load(Ordering::Relaxed),
            "Stop indexing before removing a location"
        );
        let _work = self.work.lock().unwrap();
        let mut s = self.store.lock().unwrap();
        let mut q = s.db.prepare(
            "SELECT c.id FROM chunks c JOIN files f ON f.id=c.file_id WHERE f.root_id=?1",
        )?;
        let ids = q
            .query_map([id], |r| r.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(q);
        let tx = s.db.transaction()?;
        tx.execute("DELETE FROM roots WHERE id=?1", [id])?;
        tx.commit()?;
        drop(s);
        let v = self.vectors.lock().unwrap();
        for id in ids {
            v.remove(id as u64)?
        }
        drop(v);
        self.watch()?;
        self.emit_status();
        Ok(())
    }
    pub fn update_root(&self, id: i64, exclusions: Vec<String>) -> Result<()> {
        self.store.lock().unwrap().db.execute(
            "UPDATE roots SET exclusions=?2 WHERE id=?1",
            params![id, serde_json::to_string(&exclusions)?],
        )?;
        Ok(())
    }
    pub fn start_index(&self, root: Option<i64>) -> Result<()> {
        self.running.store(true, Ordering::Relaxed);
        self.stopped.store(false, Ordering::Relaxed);
        self.paused.store(false, Ordering::Relaxed);
        let ids = if let Some(id) = root {
            vec![id]
        } else {
            self.roots()?.iter().map(|r| r.id).collect()
        };
        self.scan.lock().unwrap().extend(ids);
        self.emit_status();
        Ok(())
    }
    pub fn control(&self, action: &str) -> Result<()> {
        match action {
            "pause" => self.paused.store(true, Ordering::Relaxed),
            "resume" => {
                self.stopped.store(false, Ordering::Relaxed);
                self.paused.store(false, Ordering::Relaxed)
            }
            "stop" => {
                self.stopped.store(true, Ordering::Relaxed);
                self.running.store(false, Ordering::Relaxed)
            }
            _ => anyhow::bail!("Unknown index control"),
        };
        self.emit_status();
        Ok(())
    }
    pub fn clear(&self) -> Result<()> {
        ensure!(
            !self.running.load(Ordering::Relaxed),
            "Stop indexing before clearing the index"
        );
        let _work = self.work.lock().unwrap();
        let mut s = self.store.lock().unwrap();
        let tx = s.db.transaction()?;
        tx.execute("DELETE FROM files", [])?;
        tx.commit()?;
        drop(s);
        *self.vectors.lock().unwrap() = Vectors::new(self.settings()?.dimensions)?;
        self.scan.lock().unwrap().clear();
        self.emit_status();
        Ok(())
    }
    pub fn rebuild(&self) -> Result<()> {
        ensure!(
            !self.running.load(Ordering::Relaxed),
            "Stop indexing before rebuilding"
        );
        self.clear()?;
        self.start_index(None)
    }
    pub fn update_settings(&self, settings: Settings, rebuild: bool) -> Result<()> {
        let _configuration = self.configuration.write().unwrap();
        settings.validate()?;
        let old = self.settings()?;
        let changed = old.fingerprint() != settings.fingerprint();
        ensure!(
            !changed || rebuild,
            "This change requires semantic re-indexing confirmation"
        );
        ensure!(
            !changed || !self.running.load(Ordering::Relaxed),
            "Stop indexing before changing semantic configuration"
        );
        ensure!(
            settings.index_path == old.index_path,
            "Use the storage migration command to move the index"
        );
        let reinitialize = changed
            || old.accelerator != settings.accelerator
            || old.threads != settings.threads
            || old.profile != settings.profile;
        let _work = if reinitialize {
            Some(self.work.lock().unwrap())
        } else {
            None
        };
        let replacement = if changed {
            Some(Vectors::new(settings.dimensions)?)
        } else {
            None
        };
        let result = (|| -> Result<()> {
            if reinitialize {
                self.inference
                    .call("initialize", serde_json::to_value(&settings)?, 3)?;
            }
            self.inference
                .call("configure", serde_json::to_value(&settings)?, 3)?;
            self.store
                .lock()
                .unwrap()
                .save_configuration(&settings, changed)?;
            Ok(())
        })();
        if let Err(error) = result {
            if reinitialize {
                let _ = self
                    .inference
                    .call("initialize", serde_json::to_value(&old)?, 3);
            }
            let _ = self
                .inference
                .call("configure", serde_json::to_value(&old)?, 3);
            return Err(error);
        }
        if let Some(vectors) = replacement {
            *self.vectors.lock().unwrap() = vectors;
        }
        if changed {
            self.stopped.store(false, Ordering::Relaxed);
        }
        self.emit_status();
        (self.events)("settings-changed", serde_json::to_value(&settings)?);
        Ok(())
    }
    fn checkpoint(&self) -> bool {
        while self.paused.load(Ordering::Relaxed) && !self.stopped.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(100));
        }
        !self.stopped.load(Ordering::Relaxed)
    }
    fn excluded(path: &Path, root: &Path, settings: &Settings, extra: &[String]) -> bool {
        if !settings.sensitive_files && extract::sensitive(path) {
            return true;
        }
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
            .to_lowercase();
        settings.exclusions.iter().chain(extra).any(|x| {
            let x = x.to_lowercase().replace('\\', "/");
            if x.contains(['*', '?', '[']) {
                return glob::Pattern::new(&x).is_ok_and(|pattern| {
                    pattern.matches(&relative)
                        || path.file_name().is_some_and(|name| {
                            pattern.matches(&name.to_string_lossy().to_lowercase())
                        })
                });
            }
            !x.is_empty()
                && (relative == x
                    || relative.starts_with(&format!("{x}/"))
                    || relative.contains(&format!("/{x}/"))
                    || relative.ends_with(&format!("/{x}")))
        })
    }
    fn discover(&self, id: i64) -> Result<()> {
        let _work = self.work.lock().unwrap();
        let root = self
            .roots()?
            .into_iter()
            .find(|r| r.id == id)
            .context("Location not found")?;
        let path = PathBuf::from(&root.path);
        let online = path.exists();
        {
            let s = self.store.lock().unwrap();
            s.db.execute(
                "UPDATE roots SET online=?2 WHERE id=?1",
                params![id, online],
            )?;
            s.db.execute(
                "UPDATE files SET online=?2 WHERE root_id=?1",
                params![id, online],
            )?;
        }
        if !online {
            return Ok(());
        }
        self.status.lock().unwrap().stage = "discovery".into();
        let settings = self.settings()?;
        let seen = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let started = Instant::now();
        let journal = crate::platform::journal_checkpoint(&path);
        let mut count = 0;
        let walker = walkdir::WalkDir::new(&path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !Self::excluded(e.path(), &path, &settings, &root.exclusions));
        let mut errors = 0;
        for entry in walker {
            if !self.checkpoint() {
                return Ok(());
            }
            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    errors += 1;
                    let _ = self.store.lock().unwrap().log(
                        "skipped",
                        None,
                        err.path()
                            .map(|p| p.to_string_lossy().into_owned())
                            .as_deref()
                            .unwrap_or(""),
                        "Access denied or directory could not be read",
                    );
                    continue;
                }
            };
            if !entry.file_type().is_file()
                || entry.path().starts_with(&self.data)
                || entry.path().starts_with(&self.resources)
            {
                continue;
            }
            self.upsert_file(id, entry.path(), seen)?;
            count += 1;
            if count % 128 == 0 {
                let mut status = self.status.lock().unwrap();
                status.discovered = count;
                status.files_per_second = count as f64 / started.elapsed().as_secs_f64();
                drop(status);
                self.emit_status();
            }
        }
        // Do not tombstone unseen files when enumeration was incomplete.
        if errors == 0 {
            let mut s = self.store.lock().unwrap();
            let mut q=s.db.prepare("SELECT c.id FROM chunks c JOIN files f ON f.id=c.file_id WHERE f.root_id=?1 AND f.seen<?2")?;
            let ids = q
                .query_map(params![id, seen], |r| r.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            drop(q);
            let tx = s.db.transaction()?;
            tx.execute(
                "DELETE FROM files WHERE root_id=?1 AND seen<?2",
                params![id, seen],
            )?;
            tx.commit()?;
            drop(s);
            let vectors = self.vectors.lock().unwrap();
            for id in ids {
                vectors.remove(id as u64)?
            }
            drop(vectors);
            if let Some(journal) = journal {
                self.store.lock().unwrap().db.execute(
                    "INSERT OR REPLACE INTO settings VALUES(?1,?2)",
                    params![format!("journal:{id}"), journal],
                )?;
            }
        }
        self.store.lock().unwrap().log(
            "scan",
            None,
            &root.path,
            &format!("Discovered {count} files; {errors} unreadable locations"),
        )?;
        Ok(())
    }
    fn upsert_file(&self, root: i64, path: &Path, seen: i64) -> Result<()> {
        let metadata = std::fs::metadata(path)?;
        let path_text = display_path(path);
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let extension = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let kind = extract::kind(&extension);
        let size = metadata.len();
        let modified = metadata
            .modified()
            .ok()
            .and_then(|d| d.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let mtime_ns = metadata
            .modified()
            .ok()
            .and_then(|d| d.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);
        let created = metadata
            .created()
            .ok()
            .and_then(|d| d.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let stable = crate::file_identity::stable(path).unwrap_or_else(|| path_text.clone());
        let s = self.store.lock().unwrap();
        let renamed =
            s.db.query_row(
                "SELECT id FROM files WHERE stable_id=?1 AND root_id=?2",
                params![stable, root],
                |r| r.get::<_, i64>(0),
            )
            .ok();
        if let Some(id) = renamed {
            s.db.execute(
                "UPDATE files SET path=?2,name=?3,content_hash=CASE WHEN kind<>?4 THEN NULL ELSE content_hash END,semantic=CASE WHEN kind<>?4 THEN 0 ELSE semantic END,mtime_ns=CASE WHEN kind<>?4 THEN 0 ELSE mtime_ns END WHERE id=?1",
                params![id, path_text, name,kind],
            )?;
        }
        let old =
            s.db.query_row(
                "SELECT id,size,mtime_ns FROM files WHERE path=?1",
                [&path_text],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, u64>(1)?,
                        r.get::<_, i64>(2)?,
                    ))
                },
            )
            .ok();
        s.db.execute("INSERT INTO files(root_id,path,name,extension,kind,size,modified,created,seen,stable_id,mtime_ns) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(path) DO UPDATE SET size=excluded.size,modified=excluded.modified,mtime_ns=excluded.mtime_ns,extension=excluded.extension,kind=excluded.kind,seen=excluded.seen,online=1",params![root,path_text,name,extension,kind,size,modified,created,seen,stable,mtime_ns])?;
        let id = old.map(|x| x.0).unwrap_or_else(|| s.db.last_insert_rowid());
        if old.is_none_or(|(_, old_size, old_time)| old_size != size || old_time != mtime_ns) {
            s.db.execute(
                "INSERT OR REPLACE INTO jobs(file_id,state) VALUES(?1,'queued')",
                [id],
            )?;
        }
        Ok(())
    }
    fn index_file(&self, id: i64) -> Result<()> {
        let _work = self.work.lock().unwrap();
        let settings = self.settings()?;
        let file = {
            let s = self.store.lock().unwrap();
            s.db.execute("UPDATE jobs SET state='active' WHERE file_id=?1", [id])?;
            s.file(id)?
        };
        let path = PathBuf::from(&file.path);
        let enabled = match file.kind.as_str() {
            "text" => settings.text,
            "code" => settings.code,
            "pdf" | "document" => settings.documents,
            "image" => settings.images,
            "audio" => settings.audio,
            "video" => settings.video,
            _ => false,
        };
        if !enabled {
            let store = self.store.lock().unwrap();
            store
                .db
                .execute("DELETE FROM jobs WHERE file_id=?1", [id])?;
            store.db.execute(
                "UPDATE files SET state='metadata_only' WHERE id=?1 AND semantic=0",
                [id],
            )?;
            return Ok(());
        }
        self.status.lock().unwrap().stage = file.kind.clone();
        // Stream the content hash once; never read arbitrary binaries as text.
        let hash = {
            use std::io::Read;
            let mut f = std::fs::File::open(&path)?;
            let mut hasher = blake3::Hasher::new();
            let mut buf = [0; 65536];
            loop {
                let n = f.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
            hasher.finalize().to_hex().to_string()
        };
        let old_hash = self.store.lock().unwrap().db.query_row(
            "SELECT content_hash FROM files WHERE id=?1",
            [id],
            |r| r.get::<_, Option<String>>(0),
        )?;
        if old_hash.as_deref() == Some(&hash) && file.semantic {
            self.store
                .lock()
                .unwrap()
                .db
                .execute("DELETE FROM jobs WHERE file_id=?1", [id])?;
            return Ok(());
        }
        let mut chunks = if ["text", "code"].contains(&file.kind.as_str()) {
            ensure!(
                file.size <= settings.max_text_mb * 1024 * 1024,
                "Text exceeds the configured extraction size limit"
            );
            let bytes = std::fs::read(&path)?;
            let text = extract::decode(&bytes);
            if file.kind == "code" {
                extract::code_chunks(&text, &file.extension)
            } else {
                extract::text_chunks(&text, "text")
            }
        } else {
            let response = self.inference.call(
                "extract",
                serde_json::json!({"path":file.path,"kind":file.kind,"config":settings}),
                0,
            )?;
            let raw: Vec<Chunk> = serde_json::from_value(response.data["chunks"].clone())?;
            let mut all = Vec::new();
            for c in raw {
                if c.modality == "text" {
                    let mut parts = extract::text_chunks(&c.text, "text");
                    for p in &mut parts {
                        p.page = c.page;
                        p.heading = c.heading.clone();
                        if let Some(line) = c.line_start {
                            p.line_start = p.line_start.map(|n| n + line - 1);
                            p.line_end = p.line_end.map(|n| n + line - 1)
                        }
                    }
                    all.extend(parts)
                } else {
                    all.push(c)
                }
            }
            all
        };
        for chunk in &mut chunks {
            if !["text", "code"].contains(&chunk.modality.as_str()) {
                chunk.hash = blake3::hash(format!("{}|{hash}", chunk.hash).as_bytes())
                    .to_hex()
                    .to_string();
            }
        }
        let reuse = {
            let s = self.store.lock().unwrap();
            let mut q = s.db.prepare(
                "SELECT hash,vector FROM chunks WHERE file_id=?1 AND vector IS NOT NULL",
            )?;
            q.query_map([id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
            })?
            .collect::<rusqlite::Result<HashMap<_, _>>>()?
        };
        let mut indexed = Vec::new();
        let mut errors = Vec::new();
        let start = Instant::now();
        let mut embeddings = 0;
        let mut source = chunks.into_iter().peekable();
        while let Some(first) = source.next() {
            if !self.checkpoint() {
                self.store
                    .lock()
                    .unwrap()
                    .db
                    .execute("UPDATE jobs SET state='queued' WHERE file_id=?1", [id])?;
                return Ok(());
            }
            let batch_limit = if ["audio", "video"].contains(&first.modality.as_str())
                || settings.profile == "quiet"
            {
                1
            } else {
                4
            };
            let mut batch = vec![first];
            while batch.len() < batch_limit
                && source.peek().is_some_and(|c| {
                    c.modality == batch[0].modality
                        && c.text.len().abs_diff(batch[0].text.len()) < 1200
                })
            {
                batch.push(source.next().unwrap());
            }
            let mut vectors: Vec<Option<Vec<f32>>> = batch
                .iter()
                .map(|c| {
                    reuse.get(&c.hash).map(|b| {
                        b.as_chunks::<4>()
                            .0
                            .iter()
                            .map(|x| f32::from_le_bytes(*x))
                            .collect()
                    })
                })
                .collect();
            let missing: Vec<_> = vectors
                .iter()
                .enumerate()
                .filter(|(_, v)| v.is_none())
                .map(|(i, _)| i)
                .collect();
            if !missing.is_empty() {
                if self.inference.ready() {
                    let items:Vec<_>=missing.iter().map(|&i|{let c=&batch[i];serde_json::json!({"modality":c.modality,"text":c.text,"title":file.name,"path":c.media_path,"frames":c.frames,"start":c.start,"end":c.end})}).collect();
                    match self
                        .inference
                        .embed(serde_json::json!(items), 0, settings.dimensions)
                    {
                        Ok(output) if output.len() == missing.len() => {
                            embeddings += output.len();
                            for (i, v) in missing.into_iter().zip(output) {
                                vectors[i] = Some(v)
                            }
                        }
                        Ok(_) => errors
                            .push("Embedding batch returned an incorrect number of rows".into()),
                        Err(error) => errors.push(error.to_string()),
                    }
                } else {
                    errors.push(
                        "Semantic model unavailable; lexical content remains searchable".into(),
                    );
                }
            }
            indexed.extend(batch.into_iter().zip(vectors));
        }
        let partial = errors.first().map(String::as_str);
        let old = self
            .store
            .lock()
            .unwrap()
            .commit_chunks(id, &indexed, &hash, partial)?;
        let records = {
            let s = self.store.lock().unwrap();
            let mut q = s.db.prepare(
                "SELECT id,namespace,vector FROM chunks WHERE file_id=?1 AND vector IS NOT NULL",
            )?;
            q.query_map([id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        };
        {
            let v = self.vectors.lock().unwrap();
            for id in old {
                v.remove(id as u64)?
            }
            for (id, ns, bytes) in records {
                let vec: Vec<_> = bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|b| f32::from_le_bytes(*b))
                    .collect();
                v.add(id as u64, &ns, &vec)?
            }
        }
        self.status.lock().unwrap().embeddings_per_second =
            embeddings as f64 / start.elapsed().as_secs_f64();
        if let Some(error) = partial {
            self.store
                .lock()
                .unwrap()
                .log("partial", Some(id), &file.path, error)?;
        }
        // Decoded user media is temporary, removed after its embeddings are committed.
        self.cleanup_media(&path)?;
        Ok(())
    }
    fn cleanup_media(&self, path: &Path) -> Result<()> {
        use sha2::{Digest, Sha256};
        let hash = format!("{:x}", Sha256::digest(display_path(path).as_bytes()));
        let target = self.data.join("cache/media").join(&hash[..24]);
        if target.exists() {
            let canonical = std::fs::canonicalize(&target)?;
            let base = std::fs::canonicalize(self.data.join("cache/media"))?;
            ensure!(canonical.starts_with(base), "Invalid media cache path");
            std::fs::remove_dir_all(canonical)?
        }
        Ok(())
    }
    fn watch(self: &Arc<Self>) -> Result<()> {
        let weak = Arc::downgrade(self);
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if let (Some(e), Ok(event)) = (weak.upgrade(), event) {
                    if event.kind.is_access() {
                        return;
                    }
                    let mut changes = e.changes.lock().unwrap();
                    for path in event.paths {
                        changes.insert(path, Instant::now());
                    }
                }
            })?;
        for root in self.roots()? {
            if Path::new(&root.path).exists() {
                let _ = watcher.watch(Path::new(&root.path), RecursiveMode::Recursive);
            }
        }
        *self.watcher.lock().unwrap() = Some(watcher);
        self.status.lock().unwrap().watcher = "active".into();
        Ok(())
    }
    fn changed(&self, path: &Path) -> Result<()> {
        if path.starts_with(&self.data) || path.starts_with(&self.resources) {
            return Ok(());
        }
        let settings = self.settings()?;
        for root in self.roots()? {
            let base = PathBuf::from(&root.path);
            if path.starts_with(&base) && !Self::excluded(path, &base, &settings, &root.exclusions)
            {
                if path.is_file() {
                    self.upsert_file(
                        root.id,
                        path,
                        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
                    )?
                } else if !path.exists() {
                    let s = self.store.lock().unwrap();
                    let id =
                        s.db.query_row(
                            "SELECT id FROM files WHERE path=?1",
                            [display_path(path)],
                            |r| r.get::<_, i64>(0),
                        )
                        .ok();
                    if let Some(id) = id {
                        let chunks = s.chunks(id)?;
                        s.db.execute("DELETE FROM files WHERE id=?1", [id])?;
                        drop(s);
                        let v = self.vectors.lock().unwrap();
                        for c in chunks {
                            v.remove(c.id as u64)?
                        }
                    } else {
                        drop(s);
                        self.scan.lock().unwrap().push(root.id);
                    }
                } else {
                    self.scan.lock().unwrap().push(root.id)
                }
                break;
            }
        }
        Ok(())
    }
    pub fn status(&self) -> Result<IndexStatus> {
        let mut status = self.status.lock().unwrap().clone();
        let s = self.store.lock().unwrap();
        status.files =
            s.db.query_row("SELECT count(*) FROM files", [], |r| r.get(0))?;
        status.semantic_files =
            s.db.query_row("SELECT count(*) FROM files WHERE semantic=1", [], |r| {
                r.get(0)
            })?;
        status.errors = s.db.query_row(
            "SELECT count(*) FROM files WHERE error IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        status.chunks =
            s.db.query_row("SELECT count(*) FROM chunks", [], |r| r.get(0))?;
        status.vectors = s.db.query_row(
            "SELECT count(*) FROM chunks WHERE vector IS NOT NULL",
            [],
            |r| r.get(0),
        )?;
        status.pending =
            s.db.query_row("SELECT count(*) FROM jobs", [], |r| r.get(0))?;
        status.bytes = [
            "data/metadata.sqlite",
            "data/metadata.sqlite-wal",
            "data/metadata.sqlite-shm",
            "data/vectors/general.usearch",
            "data/vectors/code.usearch",
        ]
        .iter()
        .filter_map(|name| std::fs::metadata(self.data.join(name)).ok())
        .map(|metadata| metadata.len())
        .sum();
        status.running = self.running.load(Ordering::Relaxed);
        status.paused = self.paused.load(Ordering::Relaxed);
        status.updated = now();
        status.inference = self.inference.health.lock().unwrap().clone();
        Ok(status)
    }
    fn emit_status(&self) {
        let mut last = self.last_emit.lock().unwrap();
        if last.elapsed() < Duration::from_millis(150) {
            return;
        }
        *last = Instant::now();
        drop(last);
        if let Ok(s) = self.status() {
            (self.events)("index-progress", serde_json::to_value(s).unwrap());
        }
    }
    pub fn retry(&self, id: i64) -> Result<()> {
        let s = self.store.lock().unwrap();
        s.db.execute("INSERT OR REPLACE INTO jobs(file_id) VALUES(?1)", [id])?;
        s.db.execute(
            "UPDATE files SET error=NULL,state='queued' WHERE id=?1",
            [id],
        )?;
        self.stopped.store(false, Ordering::Relaxed);
        Ok(())
    }
    pub fn health(&self) -> Result<serde_json::Value> {
        let mut health = self.store.lock().unwrap().health()?;
        health["status"] = serde_json::to_value(self.status()?)?;
        Ok(health)
    }
    pub fn compact(&self) -> Result<()> {
        let _work = self.work.lock().unwrap();
        ensure!(
            !self.running.load(Ordering::Relaxed),
            "Stop indexing before compacting"
        );
        self.store.lock().unwrap().db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM; INSERT INTO files_fts(files_fts) VALUES('optimize'); INSERT INTO chunks_fts(chunks_fts) VALUES('optimize');")?;
        self.vectors
            .lock()
            .unwrap()
            .save(&self.data.join("data/vectors"))?;
        Ok(())
    }
    pub fn preview(&self, id: i64) -> Result<Preview> {
        let s = self.store.lock().unwrap();
        let file = s.file(id)?;
        let chunks = s.chunks(id)?;
        let text = chunks
            .iter()
            .filter(|c| !c.text.is_empty())
            .take(50)
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let asset = ["image", "audio", "video"]
            .contains(&file.kind.as_str())
            .then(|| file.path.clone());
        Ok(Preview {
            file,
            chunks,
            text,
            asset,
        })
    }
    fn file_query(&self, path: &Path, settings: &Settings) -> Result<Vec<(String, Vec<f32>)>> {
        let display = display_path(path);
        {
            let s = self.store.lock().unwrap();
            let mut q=s.db.prepare("SELECT c.namespace,c.vector FROM chunks c JOIN files f ON f.id=c.file_id WHERE f.path=?1 AND c.vector IS NOT NULL ORDER BY c.seq LIMIT 64")?;
            let rows = q.query_map([&display], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Vec<u8>>(1)?))
            })?;
            let mut vectors = Vec::new();
            for row in rows {
                let (namespace, bytes) = row?;
                let vector: Vec<f32> = bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|b| f32::from_le_bytes(*b))
                    .collect();
                if vector.len() == settings.dimensions {
                    vectors.push((namespace, vector));
                }
            }
            if !vectors.is_empty() {
                return Ok(vectors);
            }
        }
        let ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let kind = extract::kind(&ext);
        let chunks = if ["text", "code"].contains(&kind) {
            ensure!(
                std::fs::metadata(path)?.len() <= settings.max_text_mb * 1024 * 1024,
                "Query file exceeds the extraction limit"
            );
            let content = extract::decode(&std::fs::read(path)?);
            if kind == "code" {
                extract::code_chunks(&content, &ext)
            } else {
                extract::text_chunks(&content, "text")
            }
        } else {
            let resp = self.inference.call(
                "extract",
                serde_json::json!({"path":display,"kind":kind,"config":settings}),
                3,
            )?;
            serde_json::from_value::<Vec<Chunk>>(resp.data["chunks"].clone())?
        };
        ensure!(!chunks.is_empty(), "The file has no searchable content");
        let items:Vec<_>=chunks.iter().take(64).map(|c|serde_json::json!({"modality":c.modality,"text":c.text,"path":c.media_path,"frames":c.frames,"start":c.start,"end":c.end,"title":path.file_name().unwrap_or_default().to_string_lossy()})).collect();
        let vectors = self
            .inference
            .embed(serde_json::json!(items), 3, settings.dimensions);
        self.cleanup_media(path)?;
        Ok(vectors?
            .into_iter()
            .zip(chunks.iter())
            .map(|(v, c)| {
                (
                    if c.modality == "code" {
                        "code"
                    } else {
                        "general"
                    }
                    .into(),
                    v,
                )
            })
            .collect())
    }
    pub fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        let _configuration = self.configuration.read().unwrap();
        let started = Instant::now();
        self.latest_query.fetch_max(request.id, Ordering::Relaxed);
        ensure!(
            request.query.chars().count() <= 8192,
            "Search query exceeds 8192 characters"
        );
        ensure!(
            ["smart", "exact", "semantic"].contains(&request.mode.as_str()),
            "Invalid search mode"
        );
        let settings = self.settings()?;
        let (text, filters) = query::parse(&request.query, request.filters)?;
        let mut rankings: HashMap<i64, SearchResult> = HashMap::new();
        if request.mode != "semantic" && (!text.is_empty() || request.media.is_none()) {
            let s = self.store.lock().unwrap();
            let needle = text.trim_matches('"').to_lowercase();
            let pattern = format!(
                "%{}%",
                needle
                    .replace('!', "!!")
                    .replace('%', "!%")
                    .replace('_', "!_")
            );
            let mut q=s.db.prepare("SELECT id,root_id,path,name,extension,kind,size,modified,created,state,semantic,error,online FROM files WHERE lower(name) LIKE ?1 ESCAPE '!' OR lower(path) LIKE ?1 ESCAPE '!' ORDER BY modified DESC LIMIT 5000")?;
            let files = q
                .query_map([pattern], Store::read_file)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            for (rank, file) in files.into_iter().enumerate() {
                if !query::matches(&file, &filters) || !settings.show_offline && !file.online {
                    continue;
                }
                let exact = if file.name.to_lowercase() == needle {
                    1.0
                } else if file.name.to_lowercase().contains(&needle) {
                    0.3
                } else {
                    0.1
                };
                let score = query::rrf(rank, 1.) + exact;
                rankings.insert(
                    file.id,
                    SearchResult {
                        matches: s.chunks(file.id)?.into_iter().take(1).collect(),
                        file,
                        score,
                        filename: exact,
                        lexical: 0.,
                        semantic: 0.,
                    },
                );
            }
            if let Some(fts) = query::fts_query(&text) {
                let mut q=s.db.prepare("SELECT c.file_id,c.id,bm25(chunks_fts) FROM chunks_fts JOIN chunks c ON c.id=chunks_fts.rowid WHERE chunks_fts MATCH ?1 ORDER BY bm25(chunks_fts) LIMIT 5000")?;
                let rows = q
                    .query_map([fts], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                for (rank, (fid, cid)) in rows.into_iter().enumerate() {
                    let file = s.file(fid)?;
                    if !query::matches(&file, &filters) || !settings.show_offline && !file.online {
                        continue;
                    }
                    let chunk = s.chunk(cid)?;
                    let boost = if text.starts_with('"')
                        && chunk
                            .text
                            .to_lowercase()
                            .contains(&text.trim_matches('"').to_lowercase())
                    {
                        0.2
                    } else {
                        0.
                    };
                    let entry = rankings.entry(fid).or_insert(SearchResult {
                        file,
                        matches: Vec::new(),
                        score: 0.,
                        filename: 0.,
                        lexical: 0.,
                        semantic: 0.,
                    });
                    if entry.lexical == 0. {
                        entry.lexical = query::rrf(rank, 1.);
                        entry.score += entry.lexical + boost;
                        entry.matches.clear();
                    }
                    if entry.matches.len() < 3 {
                        entry.matches.push(chunk)
                    }
                }
            }
        }
        let mut warning = None;
        if request.semantic_pass
            && request.mode != "exact"
            && (self.inference.ready() || self.inference.health.lock().unwrap()["state"] == "idle")
        {
            if self.latest_query.load(Ordering::Relaxed) > request.id {
                return Ok(SearchResponse {
                    id: request.id,
                    results: Vec::new(),
                    elapsed_ms: started.elapsed().as_secs_f64() * 1000.,
                    semantic_ready: true,
                    warning: None,
                });
            }
            let queries = if let Some(path) = request.media {
                match self.file_query(&std::fs::canonicalize(path)?, &settings) {
                    Ok(vectors) => vectors,
                    Err(error) => {
                        warning = Some(error.to_string());
                        Vec::new()
                    }
                }
            } else if !text.is_empty() {
                match self.inference.embed(serde_json::json!([{"modality":"text","query":true,"text":text.trim_matches('"')},{"modality":"code","query":true,"text":text.trim_matches('"')}]),3,settings.dimensions){Ok(mut vs) if vs.len()==2=>{let code=vs.pop().unwrap();vec![("general".into(),vs.pop().unwrap()),("code".into(),code)]},Err(e)=>{warning=Some(e.to_string());Vec::new()},_=>Vec::new()}
            } else {
                Vec::new()
            };
            for (namespace, v) in queries {
                let neighbors = self.vectors.lock().unwrap().search(
                    &v,
                    &namespace,
                    (settings.results_count * 8).max(256),
                )?;
                let s = self.store.lock().unwrap();
                for (rank, (cid, distance)) in neighbors.into_iter().enumerate() {
                    let fid =
                        s.db.query_row(
                            "SELECT file_id FROM chunks WHERE id=?1",
                            [cid as i64],
                            |r| r.get::<_, i64>(0),
                        )
                        .ok();
                    let Some(fid) = fid else { continue };
                    let file = s.file(fid)?;
                    if !query::matches(&file, &filters) || !settings.show_offline && !file.online {
                        continue;
                    }
                    let chunk = s.chunk(cid as i64)?;
                    let entry = rankings.entry(fid).or_insert(SearchResult {
                        file,
                        matches: Vec::new(),
                        score: 0.,
                        filename: 0.,
                        lexical: 0.,
                        semantic: 0.,
                    });
                    let score = query::rrf(rank, 1.2);
                    if entry.semantic == 0. {
                        entry.score += score;
                        entry.semantic = (1. - distance) as f64;
                        if entry.lexical == 0. {
                            entry.matches.clear()
                        }
                    }
                    if entry.matches.len() < 3 && !entry.matches.iter().any(|c| c.id == chunk.id) {
                        entry.matches.push(chunk)
                    }
                }
            }
        } else if request.semantic_pass && request.mode != "exact" {
            warning = Some(
                "Semantic engine is not ready; filename and text search remain available".into(),
            );
        }
        let mut results: Vec<_> = rankings.into_values().collect();
        results.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then(b.file.modified.cmp(&a.file.modified))
        });
        results.truncate(settings.results_count);
        Ok(SearchResponse {
            id: request.id,
            results,
            elapsed_ms: started.elapsed().as_secs_f64() * 1000.,
            semantic_ready: self.inference.ready(),
            warning,
        })
    }
    pub fn hardware() -> serde_json::Value {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        serde_json::json!({"cpu":sys.cpus().first().map(|c|c.brand()).unwrap_or("Unknown"),"threads":sys.cpus().len(),"ram_bytes":sys.total_memory(),"available_ram_bytes":sys.available_memory(),"os":sysinfo::System::long_os_version(),"fingerprint":blake3::hash(format!("{:?}|{}",sys.cpus().first().map(|c|c.brand()),sys.total_memory()).as_bytes()).to_hex().to_string()})
    }
}
pub fn display_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    if let Some(p) = s.strip_prefix("\\\\?\\UNC\\") {
        format!("\\\\{p}")
    } else {
        s.strip_prefix("\\\\?\\").unwrap_or(&s).to_string()
    }
}
