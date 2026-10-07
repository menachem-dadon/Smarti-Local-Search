use smarti_search_core::{Engine, store::Store, types::*};
use std::{
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
fn wait(engine: &Engine, predicate: impl Fn(&IndexStatus) -> bool) {
    let start = Instant::now();
    loop {
        let status = engine.status().unwrap();
        if predicate(&status) {
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(180),
            "Timed out: {status:?}"
        );
        thread::sleep(Duration::from_millis(100));
    }
}
#[test]
#[ignore = "requires bundled model and runtimes; run in release validation"]
fn real_corpus_incremental_restart() {
    let directory = tempfile::tempdir().unwrap();
    let data = directory.path().join("data");
    let corpus = directory.path().join("corpus");
    std::fs::create_dir_all(&corpus).unwrap();
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for item in std::fs::read_dir(repository.join("tests/fixtures/corpus")).unwrap() {
        let item = item.unwrap();
        std::fs::copy(item.path(), corpus.join(item.file_name())).unwrap();
    }
    let settings = Settings {
        accelerator: "cpu".into(),
        ..Default::default()
    };
    let store = Store::open(&data.join("data/metadata.sqlite")).unwrap();
    store.save_settings(&settings).unwrap();
    drop(store);
    let e = Engine::open(
        data.clone(),
        repository.join("resources"),
        Arc::new(|_, _| {}),
    )
    .unwrap();
    wait(&e, |s| s.inference["ready"] == true);
    e.add_root(&corpus).unwrap();
    e.start_index(None).unwrap();
    wait(&e, |s| s.files >= 11 && s.pending == 0 && !s.running);
    let status = e.status().unwrap();
    assert!(status.semantic_files >= 10, "{status:?}");
    assert_eq!(
        status.errors,
        0,
        "{:?}",
        e.store.lock().unwrap().activity().unwrap()
    );
    let request = |id, query: &str, mode: &str, media: Option<String>| SearchRequest {
        id,
        query: query.into(),
        mode: mode.into(),
        filters: Filters::default(),
        media,
        semantic_pass: mode != "exact",
    };
    let exact = e.search(request(1, "auth.rs", "exact", None)).unwrap();
    assert_eq!(exact.results[0].file.name, "auth.rs");
    let lexical = e.search(request(2, "SQLite", "exact", None)).unwrap();
    assert!(!lexical.results.is_empty());
    let semantic = e
        .search(request(3, "a child playing by the sea", "smart", None))
        .unwrap();
    assert!(
        semantic
            .results
            .iter()
            .take(4)
            .any(|r| r.file.name == "ocean.txt" || r.file.name == "beach.png")
    );
    let code = e
        .search(request(4, "connecting an OAuth token", "semantic", None))
        .unwrap();
    assert!(
        code.results
            .iter()
            .take(5)
            .any(|r| r.file.name == "auth.rs")
    );
    let image = e
        .search(request(
            5,
            "",
            "smart",
            Some(corpus.join("beach.png").to_string_lossy().into_owned()),
        ))
        .unwrap();
    assert!(
        image
            .results
            .iter()
            .take(3)
            .any(|r| r.file.name == "beach.png")
    );
    assert!(
        e.search(request(6, ".env", "exact", None))
            .unwrap()
            .results
            .is_empty()
    );
    let old_id = exact.results[0].file.id;
    let original_chunks: Vec<(i64, i64)> = e
        .store
        .lock()
        .unwrap()
        .db
        .prepare("SELECT file_id,id FROM chunks ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    std::fs::rename(corpus.join("auth.rs"), corpus.join("renamed.rs")).unwrap();
    wait(&e, |_| {
        e.store
            .lock()
            .unwrap()
            .db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM files WHERE name='renamed.rs')",
                [],
                |row| row.get::<_, bool>(0),
            )
            .unwrap()
    });
    wait(&e, |s| !s.running && s.pending == 0);
    thread::sleep(Duration::from_secs(1));
    let renamed = e.search(request(7, "renamed.rs", "exact", None)).unwrap();
    assert_eq!(renamed.results[0].file.id, old_id);
    assert_eq!(
        original_chunks,
        e.store
            .lock()
            .unwrap()
            .db
            .prepare("SELECT file_id,id FROM chunks ORDER BY id")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<rusqlite::Result<Vec<(i64, i64)>>>()
            .unwrap()
    );
    std::fs::write(
        corpus.join("ocean.txt"),
        "Changed document about a mountain observatory.",
    )
    .unwrap();
    wait(&e, |_| {
        e.store
            .lock()
            .unwrap()
            .db
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM chunks WHERE text LIKE '%mountain observatory%')",
                [],
                |row| row.get::<_, bool>(0),
            )
            .unwrap()
    });
    wait(&e, |s| s.pending == 0 && !s.running);
    thread::sleep(Duration::from_secs(1));
    assert!(
        !e.search(request(8, "observatory", "exact", None))
            .unwrap()
            .results
            .is_empty()
    );
    let ocean_id: i64 = e
        .store
        .lock()
        .unwrap()
        .db
        .query_row("SELECT id FROM files WHERE name='ocean.txt'", [], |row| {
            row.get(0)
        })
        .unwrap();
    let after_chunks: Vec<(i64, i64)> = e
        .store
        .lock()
        .unwrap()
        .db
        .prepare("SELECT file_id,id FROM chunks ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(
        original_chunks
            .into_iter()
            .filter(|(file, _)| *file != ocean_id)
            .collect::<Vec<_>>(),
        after_chunks
            .iter()
            .copied()
            .filter(|(file, _)| *file != ocean_id)
            .collect::<Vec<_>>(),
        "editing one file must preserve every other content chunk/vector"
    );
    let scans: i64 = e
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
    assert_eq!(scans, 1, "file edit/rename must not schedule a root scan");
    e.start_index(None).unwrap();
    wait(&e, |s| !s.running && s.pending == 0);
    let rescanned: Vec<(i64, i64)> = e
        .store
        .lock()
        .unwrap()
        .db
        .prepare("SELECT file_id,id FROM chunks ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(
        after_chunks, rescanned,
        "explicit metadata reconciliation must not re-index unchanged content"
    );
    std::fs::rename(corpus.join("renamed.rs"), corpus.join("renamed.bin")).unwrap();
    wait(&e, |_| {
        e.store.lock().unwrap().db.query_row("SELECT EXISTS(SELECT 1 FROM files WHERE id=?1 AND kind='binary' AND state='metadata_only' AND semantic=0)",[old_id],|row| row.get::<_,bool>(0)).unwrap()
    });
    assert!(
        e.store.lock().unwrap().chunks(old_id).unwrap().is_empty(),
        "changed file type must discard stale content"
    );
    std::fs::rename(corpus.join("renamed.bin"), corpus.join("renamed.rs")).unwrap();
    wait(&e, |_| {
        e.store
            .lock()
            .unwrap()
            .db
            .query_row(
                "SELECT semantic=1 FROM files WHERE id=?1",
                [old_id],
                |row| row.get::<_, bool>(0),
            )
            .unwrap()
    });
    e.control("pause").unwrap();
    assert!(e.status().unwrap().paused);
    e.control("resume").unwrap();
    e.control("stop").unwrap();
    drop(e);
    thread::sleep(Duration::from_secs(1));
    let e = Engine::open(data, repository.join("resources"), Arc::new(|_, _| {})).unwrap();
    assert!(e.status().unwrap().files >= 11);
    assert!(!e.status().unwrap().running);
    assert_eq!(e.store.lock().unwrap().health().unwrap()["integrity"], "ok");
    wait(&e, |s| s.inference["ready"] == true);
    e.control("stop").unwrap();
    e.control("pause").unwrap();
    let changed = Settings {
        dimensions: 128,
        ..e.settings().unwrap()
    };
    assert!(e.update_settings(changed.clone(), false).is_err());
    e.store.lock().unwrap().db.execute_batch("CREATE TRIGGER fail_rebuild BEFORE UPDATE OF semantic ON files BEGIN SELECT RAISE(ABORT,'Injected rebuild failure'); END;").unwrap();
    assert!(e.update_settings(changed.clone(), true).is_err());
    assert_eq!(e.settings().unwrap().dimensions, 256);
    assert_eq!(e.status().unwrap().inference["dimensions"], 256);
    assert!(e.status().unwrap().semantic_files >= 10);
    assert!(
        !e.search(request(9, "connecting an OAuth token", "semantic", None))
            .unwrap()
            .results
            .is_empty()
    );
    e.store
        .lock()
        .unwrap()
        .db
        .execute_batch("DROP TRIGGER fail_rebuild")
        .unwrap();
    e.update_settings(changed, true).unwrap();
    assert_eq!(e.settings().unwrap().dimensions, 128);
    assert_eq!(e.status().unwrap().semantic_files, 0);
    assert!(e.status().unwrap().pending >= 10);
    assert!(
        !e.search(request(10, "observatory", "exact", None))
            .unwrap()
            .results
            .is_empty()
    );
    e.control("resume").unwrap();
    wait(&e, |s| s.pending == 0 && !s.running);
    assert!(e.status().unwrap().semantic_files >= 10);
    assert_eq!(e.status().unwrap().errors, 0);
    assert!(
        !e.search(request(11, "connecting an OAuth token", "semantic", None))
            .unwrap()
            .results
            .is_empty()
    );
    e.control("stop").unwrap();
    drop(e);
}
