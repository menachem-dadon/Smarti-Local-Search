use rusqlite::params;
use smarti_search_core::{
    store::Store,
    types::{Chunk, Settings},
};
#[test]
fn atomic_update_resume_and_delete() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("metadata.sqlite");
    let mut s = Store::open(&path).unwrap();
    let root = s.add_root("C:\\Synthetic").unwrap();
    s.db.execute("INSERT INTO files(root_id,path,name,extension,kind,size,modified,created,stable_id) VALUES(?1,'C:\\Synthetic\\one.txt','one.txt','txt','text',1,1,1,'1:1')",[root]).unwrap();
    let fid = s.db.last_insert_rowid();
    s.db.execute("INSERT INTO jobs VALUES(?1,'active')", [fid])
        .unwrap();
    let chunk = Chunk {
        modality: "text".into(),
        text: "שלום local architecture".into(),
        hash: "first".into(),
        ..Default::default()
    };
    s.commit_chunks(fid, &[(chunk, Some(vec![1.; 128]))], "hash", None)
        .unwrap();
    assert_eq!(s.chunks(fid).unwrap().len(), 1);
    assert_eq!(s.health().unwrap()["integrity"], "ok");
    s.db.execute(
        "UPDATE files SET path='C:\\Synthetic\\renamed.txt',name='renamed.txt' WHERE id=?1",
        [fid],
    )
    .unwrap();
    let name: String =
        s.db.query_row(
            "SELECT name FROM files_fts WHERE files_fts MATCH 'renamed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(name, "renamed.txt");
    s.db.execute("INSERT INTO jobs VALUES(?1,'active')", [fid])
        .unwrap();
    drop(s);
    let s = Store::open(&path).unwrap();
    let state: String =
        s.db.query_row("SELECT state FROM jobs WHERE file_id=?1", [fid], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(state, "queued");
    s.db.execute("DELETE FROM roots WHERE id=?1", [root])
        .unwrap();
    let count: i64 =
        s.db.query_row("SELECT count(*) FROM chunks", [], |r| r.get(0))
            .unwrap();
    assert_eq!(count, 0);
    let fts: i64 =
        s.db.query_row(
            "SELECT count(*) FROM chunks_fts WHERE chunks_fts MATCH 'architecture'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(fts, 0);
}
#[test]
fn settings_validation_and_migration_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::open(&dir.path().join("test.sqlite")).unwrap();
    s.db.execute(
        "INSERT INTO settings VALUES('app',?1)",
        params!["{\"theme\":\"dark\"}"],
    )
    .unwrap();
    let settings = s.settings().unwrap();
    assert_eq!(settings.theme, "dark");
    assert_eq!(settings.dimensions, 256);
    let invalid = Settings {
        dimensions: 999,
        ..settings
    };
    assert!(s.save_settings(&invalid).is_err());
}

#[test]
fn semantic_configuration_and_rebuild_queue_commit_or_rollback_together() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("test.sqlite")).unwrap();
    let old = Settings::default();
    store.save_settings(&old).unwrap();
    let root = store.add_root("C:\\Synthetic").unwrap();
    store.db.execute("INSERT INTO files(root_id,path,name,extension,kind,size,modified,created) VALUES(?1,'C:\\Synthetic\\one.txt','one.txt','txt','text',1,1,1)", [root]).unwrap();
    let file = store.db.last_insert_rowid();
    let chunk = Chunk {
        modality: "text".into(),
        text: "persistent architecture".into(),
        hash: "original".into(),
        ..Default::default()
    };
    store
        .commit_chunks(
            file,
            &[(chunk, Some(vec![1.; old.dimensions]))],
            "hash",
            None,
        )
        .unwrap();
    let changed = Settings {
        dimensions: 128,
        ..old.clone()
    };
    // Fail after vectors were cleared, before the remaining rebuild statements.
    store.db.execute_batch("CREATE TRIGGER fail_rebuild BEFORE UPDATE OF semantic ON files BEGIN SELECT RAISE(ABORT,'Injected rebuild failure'); END;").unwrap();
    assert!(store.save_configuration(&changed, true).is_err());
    assert_eq!(store.settings().unwrap().dimensions, old.dimensions);
    assert!(store.file(file).unwrap().semantic);
    let length: usize = store
        .db
        .query_row(
            "SELECT length(vector) FROM chunks WHERE file_id=?1",
            [file],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(length, old.dimensions * 4);
    let jobs: usize = store
        .db
        .query_row("SELECT count(*) FROM jobs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(jobs, 0);
    store.db.execute_batch("DROP TRIGGER fail_rebuild").unwrap();
    store.save_configuration(&changed, true).unwrap();
    assert_eq!(store.settings().unwrap().dimensions, 128);
    assert!(!store.file(file).unwrap().semantic);
    let vector: Option<Vec<u8>> = store
        .db
        .query_row(
            "SELECT vector FROM chunks WHERE file_id=?1",
            [file],
            |row| row.get(0),
        )
        .unwrap();
    assert!(vector.is_none());
    let state: String = store
        .db
        .query_row("SELECT state FROM jobs WHERE file_id=?1", [file], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(state, "queued");
    let matches: usize = store
        .db
        .query_row(
            "SELECT count(*) FROM chunks_fts WHERE chunks_fts MATCH 'architecture'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(matches, 1);
}
