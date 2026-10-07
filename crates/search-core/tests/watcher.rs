use smarti_search_core::{Engine, store::Store, types::Settings};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

fn wait(engine: &Engine, files: u64) {
    let start = Instant::now();
    loop {
        let status = engine.status().unwrap();
        if status.files == files && status.pending == 0 && !status.running {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(20), "{status:?}");
        std::thread::sleep(Duration::from_millis(40));
    }
    // Allow debounced duplicate native events to arrive before assertions.
    std::thread::sleep(Duration::from_millis(800));
}
fn path_id(engine: &Engine, path: &Path) -> i64 {
    engine
        .store
        .lock()
        .unwrap()
        .db
        .query_row(
            "SELECT id FROM files WHERE path=?1",
            [smarti_search_core::engine::display_path(
                &std::fs::canonicalize(path).unwrap(),
            )],
            |row| row.get(0),
        )
        .unwrap()
}
fn queued(engine: &Engine) -> Vec<i64> {
    engine
        .store
        .lock()
        .unwrap()
        .db
        .prepare("SELECT file_id FROM test_jobs ORDER BY rowid")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

#[test]
fn native_watcher_updates_only_changed_files_and_moves_or_deletes_subtrees() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("corpus");
    let data = temp.path().join("index");
    let resources = temp.path().join("resources");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&resources).unwrap();
    for id in 0..200 {
        std::fs::write(root.join(format!("{id}.bin")), b"unchanged").unwrap();
    }
    let store = Store::open(&data.join("data/metadata.sqlite")).unwrap();
    store.save_settings(&Settings::default()).unwrap();
    drop(store);
    let engine = Engine::open(data, resources, Arc::new(|_, _| {})).unwrap();
    engine.add_root(&root).unwrap();
    engine.control("pause").unwrap();
    // Multiple identical requests, before execution, produce one root scan.
    for _ in 0..12 {
        engine.start_index(None).unwrap();
        engine.control("pause").unwrap();
    }
    engine.control("resume").unwrap();
    wait(&engine, 200);
    engine.store.lock().unwrap().db.execute_batch("CREATE TABLE test_jobs(file_id INTEGER);
        CREATE TRIGGER test_queue AFTER INSERT ON jobs BEGIN INSERT INTO test_jobs VALUES(new.file_id); END;").unwrap();
    let edited = root.join("4.bin");
    let id = path_id(&engine, &edited);
    for _ in 0..100 {
        std::fs::write(&edited, b"changed!!").unwrap();
    }
    wait(&engine, 200);
    assert_eq!(
        queued(&engine),
        vec![id],
        "duplicate modify events must coalesce"
    );
    engine
        .store
        .lock()
        .unwrap()
        .db
        .execute("DELETE FROM test_jobs", [])
        .unwrap();
    let folder = root.join("new-folder");
    std::fs::create_dir(&folder).unwrap();
    for id in 0..20 {
        std::fs::write(folder.join(format!("{id}.bin")), b"new").unwrap();
    }
    wait(&engine, 220);
    let new_jobs = queued(&engine);
    assert_eq!(new_jobs.len(), 20);
    assert!(new_jobs.iter().all(|id| *id > 200));
    engine
        .store
        .lock()
        .unwrap()
        .db
        .execute("DELETE FROM test_jobs", [])
        .unwrap();
    let child_id = path_id(&engine, &folder.join("3.bin"));
    let moved = root.join("moved-folder");
    std::fs::rename(&folder, &moved).unwrap();
    wait(&engine, 220);
    assert_eq!(path_id(&engine, &moved.join("3.bin")), child_id);
    assert!(
        queued(&engine).is_empty(),
        "unchanged renamed content must not be requeued"
    );
    let ghost = root.join("temporary.bin");
    std::fs::write(&ghost, b"gone before debounce").unwrap();
    std::fs::remove_file(&ghost).unwrap();
    let cache = root.join(".cache");
    std::fs::create_dir(&cache).unwrap();
    std::fs::write(cache.join("ignored.bin"), b"excluded").unwrap();
    std::fs::remove_dir_all(&moved).unwrap();
    wait(&engine, 200);
    assert!(queued(&engine).is_empty());
    let scans: i64 = engine
        .store
        .lock()
        .unwrap()
        .db
        .query_row(
            "SELECT count(*) FROM activity WHERE kind='scan'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        scans, 1,
        "ordinary folder events must never rescan the root"
    );
    engine.control("stop").unwrap();
    std::fs::write(&edited, b"changed while stopped").unwrap();
    std::thread::sleep(Duration::from_millis(850));
    assert!(queued(&engine).is_empty());
    engine.control("resume").unwrap();
    wait(&engine, 200);
    assert_eq!(queued(&engine), vec![id]);
    engine.control("stop").unwrap();
}
