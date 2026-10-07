use smarti_search_core::{Engine, store::Store, types::Settings};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[test]
fn large_metadata_queue_finishes_without_per_file_sleep_and_applies_path_exclusions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("corpus");
    let hidden = root.join("private");
    let data = temp.path().join("index");
    let resources = temp.path().join("resources");
    std::fs::create_dir_all(&hidden).unwrap();
    std::fs::create_dir_all(&resources).unwrap();
    for id in 0..2000 {
        std::fs::write(root.join(format!("file-{id}.bin")), b"fixture").unwrap();
    }
    std::fs::write(hidden.join("secret.bin"), b"excluded").unwrap();
    let store = Store::open(&data.join("data/metadata.sqlite")).unwrap();
    store.save_settings(&Settings::default()).unwrap();
    store
        .db
        .execute("INSERT INTO settings VALUES('index_stopped','1')", [])
        .unwrap();
    drop(store);
    let engine = Engine::open(data, resources, Arc::new(|_, _| {})).unwrap();
    engine.add_root(&root).unwrap();
    let mut settings = engine.settings().unwrap();
    settings
        .exclusions
        .push(hidden.to_string_lossy().into_owned());
    engine.update_settings(settings.clone(), false).unwrap();
    std::thread::sleep(Duration::from_millis(350));
    assert_eq!(
        engine.status().unwrap().files,
        0,
        "a stopped index must stay stopped after exclusion edits"
    );
    engine.start_index(None).unwrap();
    let start = Instant::now();
    loop {
        let status = engine.status().unwrap();
        if status.files == 2000 && status.pending == 0 && !status.running {
            assert!(status.discovery_complete);
            assert_eq!(status.discovery_total, 2000);
            assert_eq!(status.discovery_processed, 2000);
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(25),
            "metadata processing took too long: {status:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    engine.control("stop").unwrap();
    assert_eq!(
        engine
            .store
            .lock()
            .unwrap()
            .db
            .query_row(
                "SELECT value FROM settings WHERE key='index_stopped'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "1"
    );
    settings
        .exclusions
        .retain(|rule| rule != &hidden.to_string_lossy());
    engine.update_settings(settings.clone(), false).unwrap();
    engine.start_index(None).unwrap();
    loop {
        let status = engine.status().unwrap();
        if status.files == 2001 && status.pending == 0 && !status.running {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(30), "{status:?}");
        std::thread::sleep(Duration::from_millis(25));
    }
    engine.control("stop").unwrap();
    settings
        .exclusions
        .push(hidden.to_string_lossy().into_owned());
    engine.update_settings(settings, false).unwrap();
    engine.start_index(None).unwrap();
    let prune_started = Instant::now();
    loop {
        let status = engine.status().unwrap();
        if status.files == 2000 && status.pending == 0 && !status.running {
            break;
        }
        assert!(
            prune_started.elapsed() < Duration::from_secs(15),
            "{status:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(
        hidden.join("secret.bin").exists(),
        "exclusions must never delete source files"
    );
    engine.control("stop").unwrap();
}
