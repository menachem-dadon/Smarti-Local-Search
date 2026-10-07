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
    s.db.execute("INSERT INTO jobs(file_id,state) VALUES(?1,'active')", [fid])
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
    s.db.execute("INSERT INTO jobs(file_id,state) VALUES(?1,'active')", [fid])
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
fn upgrade_adds_defaults_once_and_keeps_user_choices() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.sqlite");
    let store = Store::open(&path).unwrap();
    let settings = Settings {
        theme: "dark".into(),
        exclusions: vec!["my-custom-folder".into()],
        ..Default::default()
    };
    store.save_settings(&settings).unwrap();
    store
        .db
        .execute("DELETE FROM settings WHERE key='exclusions_v2'", [])
        .unwrap();
    drop(store);
    let store = Store::open(&path).unwrap();
    let mut upgraded = store.settings().unwrap();
    assert_eq!(upgraded.theme, "dark");
    assert!(upgraded.exclusions.contains(&"my-custom-folder".into()));
    assert!(upgraded.exclusions.contains(&".cache".into()));
    upgraded.exclusions.retain(|rule| rule != ".cache");
    store.save_settings(&upgraded).unwrap();
    drop(store);
    assert!(
        !Store::open(&path)
            .unwrap()
            .settings()
            .unwrap()
            .exclusions
            .contains(&".cache".into())
    );
}

#[test]
fn identity_and_priority_queries_use_indices_and_preserve_queue_order() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("test.sqlite")).unwrap();
    let root = store.add_root("C:/Synthetic").unwrap();
    for (id, kind) in [(1, "video"), (2, "image"), (3, "text"), (4, "code")] {
        store
            .db
            .execute(
                "INSERT INTO files(id,root_id,path,name,kind,stable_id) VALUES(?1,?2,?3,?3,?4,?3)",
                params![id, root, format!("file-{id}"), kind],
            )
            .unwrap();
        store
            .db
            .execute("INSERT INTO jobs(file_id) VALUES(?1)", [id])
            .unwrap();
    }
    let order = || {
        store
            .db
            .prepare("SELECT file_id FROM jobs WHERE state='queued' ORDER BY priority,file_id")
            .unwrap()
            .query_map([], |r| r.get::<_, i64>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    };
    assert_eq!(order(), vec![3, 4, 2, 1]);
    store
        .db
        .execute("UPDATE files SET kind='text' WHERE id=1", [])
        .unwrap();
    assert_eq!(order(), vec![1, 3, 4, 2]);
    let plan: String = store
        .db
        .query_row(
            "EXPLAIN QUERY PLAN SELECT id FROM files WHERE root_id=1 AND stable_id='one'",
            [],
            |r| r.get(3),
        )
        .unwrap();
    assert!(plan.contains("files_identity"), "{plan}");
    let plans=store.db.prepare("EXPLAIN QUERY PLAN SELECT f.id FROM jobs j INDEXED BY jobs_dispatch JOIN files f ON f.id=j.file_id WHERE j.state='queued' AND f.online=1 ORDER BY j.priority,j.file_id LIMIT 1").unwrap().query_map([],|r|r.get::<_,String>(3)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    assert!(
        plans.iter().any(|p| p.contains("jobs_dispatch")),
        "{plans:?}"
    );
    assert!(
        !plans.iter().any(|p| p.contains("TEMP B-TREE")),
        "{plans:?}"
    );
}

#[test]
fn upgrades_the_legacy_two_column_queue_without_dropping_jobs() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.sqlite");
    let store = Store::open(&path).unwrap();
    let root = store.add_root("C:/Synthetic").unwrap();
    store
        .db
        .execute(
            "INSERT INTO files(id,root_id,path,name,kind) VALUES(1,?1,'one.txt','one.txt','text')",
            [root],
        )
        .unwrap();
    store.db.execute_batch("DROP TRIGGER jobs_priority; DROP TRIGGER files_job_priority; DROP TABLE jobs;
        CREATE TABLE jobs(file_id INTEGER PRIMARY KEY REFERENCES files(id) ON DELETE CASCADE,state TEXT NOT NULL DEFAULT 'queued');
        INSERT INTO jobs VALUES(1,'active');").unwrap();
    drop(store);
    let upgraded = Store::open(&path).unwrap();
    let job: (i64, String, i64) = upgraded
        .db
        .query_row("SELECT file_id,state,priority FROM jobs", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert_eq!(job, (1, "queued".into(), 0));
    assert_eq!(upgraded.health().unwrap()["integrity"], "ok");
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
