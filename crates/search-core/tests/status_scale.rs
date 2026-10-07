use smarti_search_core::{Engine, store::Store, types::Settings};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[test]
fn status_on_150000_files_uses_indices_and_avoids_content_rows() {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("index");
    let resources = temp.path().join("resources");
    std::fs::create_dir(&resources).unwrap();
    let store = Store::open(&data.join("data/metadata.sqlite")).unwrap();
    store.save_settings(&Settings::default()).unwrap();
    store
        .db
        .execute("INSERT INTO settings VALUES('index_stopped','1')", [])
        .unwrap();
    store.add_root("C:/Synthetic").unwrap();
    store
        .db
        .execute_batch(
            "BEGIN;
        WITH RECURSIVE n(id) AS (SELECT 1 UNION ALL SELECT id+1 FROM n WHERE id<150000)
        INSERT INTO files(id,root_id,path,name,kind,semantic,error) SELECT id,1,
            'C:/Synthetic/'||printf('%0600d',id),printf('%0600d',id),
            CASE WHEN id%2=0 THEN 'text' ELSE 'image' END,id%3=0,
            CASE WHEN id%500=0 THEN 'fixture error' END FROM n;
        INSERT INTO jobs(file_id) SELECT id FROM files;
        COMMIT;",
        )
        .unwrap();
    drop(store);
    let engine = Engine::open(data, resources, Arc::new(|_, _| {})).unwrap();
    // Seed content after opening: this is a status-query benchmark, independent
    // of ANN restoration/inference. Large BLOBs must never enter these queries.
    let store = engine.store.lock().unwrap();
    store.db.execute("INSERT INTO chunks(file_id,seq,text,vector) SELECT id,0,'fixture',zeroblob(1024) FROM files WHERE id%3=0", []).unwrap();
    for (sql, index) in [
        (
            "SELECT count(*) FROM files WHERE semantic=1",
            "files_semantic",
        ),
        (
            "SELECT count(*) FROM files WHERE error IS NOT NULL",
            "files_errors",
        ),
        (
            "SELECT count(*) FROM chunks WHERE vector IS NOT NULL",
            "chunks_vectors",
        ),
        (
            "SELECT kind,count(*) FROM jobs WHERE online=1 GROUP BY kind",
            "jobs_summary",
        ),
    ] {
        let plans = store
            .db
            .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
            .unwrap()
            .query_map([], |row| row.get::<_, String>(3))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(plans.iter().any(|plan| plan.contains(index)), "{plans:?}");
        assert!(
            !plans.iter().any(|plan| plan.contains("TEMP B-TREE")),
            "{plans:?}"
        );
    }
    drop(store);
    let warm = engine.status().unwrap();
    assert_eq!(
        (
            warm.files,
            warm.semantic_files,
            warm.errors,
            warm.pending,
            warm.vectors
        ),
        (150000, 50000, 300, 150000, 50000)
    );
    let start = Instant::now();
    for _ in 0..20 {
        assert_eq!(engine.status().unwrap().pending, 150000);
    }
    let elapsed = start.elapsed();
    println!(
        "150000-file status: {:.2} ms per refresh",
        elapsed.as_secs_f64() * 1000. / 20.
    );
    assert!(
        elapsed < Duration::from_secs(2),
        "Status must remain below 100 ms per refresh: {elapsed:?}"
    );
    engine.control("stop").unwrap();
}
