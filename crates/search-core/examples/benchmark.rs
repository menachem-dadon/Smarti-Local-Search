use anyhow::Result;
use smarti_search_core::{
    store::Store,
    vectors::{Vectors, normalize},
};
use std::{path::PathBuf, time::Instant};
fn sample(mut seed: u64) -> Vec<f32> {
    seed += 1;
    let mut values = Vec::with_capacity(128);
    for _ in 0..128 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        values.push(((seed >> 32) as u32) as f32 / u32::MAX as f32 - 0.5);
    }
    normalize(values, 128).unwrap()
}
fn memory() -> u64 {
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    system
        .process(sysinfo::get_current_pid().unwrap())
        .map(|p| p.memory())
        .unwrap_or(0)
}
fn main() -> Result<()> {
    let output = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "artifacts/performance".into()),
    );
    std::fs::create_dir_all(&output)?;
    let count = 100_000;
    let start = Instant::now();
    let index = Vectors::new(128)?;
    index.general.reserve(count)?;
    for id in 0..count {
        index.add(id as u64 + 1, "general", &sample(id as u64))?;
    }
    let build_ms = start.elapsed().as_secs_f64() * 1000.;
    let resident = memory();
    let mut samples = Vec::new();
    for i in 0..100 {
        let start = Instant::now();
        let result = index.search(&sample(i * 997), "general", 20)?;
        anyhow::ensure!(!result.is_empty(), "ANN returned no candidates");
        samples.push(start.elapsed().as_secs_f64() * 1000.);
    }
    samples.sort_by(f64::total_cmp);
    let start = Instant::now();
    index.save(&output.join("ann"))?;
    let save_ms = start.elapsed().as_secs_f64() * 1000.;
    drop(index);
    let start = Instant::now();
    let loaded = Vectors::new(128)?;
    loaded.load(&output.join("ann"))?;
    let load_ms = start.elapsed().as_secs_f64() * 1000.;
    anyhow::ensure!(loaded.general.size() == count, "ANN load lost vectors");
    let store = Store::open(&output.join("benchmark.sqlite"))?;
    store.db.execute("DELETE FROM files", [])?;
    store.db.execute(
        "INSERT OR IGNORE INTO roots(id,path) VALUES(1,'synthetic')",
        [],
    )?;
    store.db.execute_batch("BEGIN")?;
    let start = Instant::now();
    {
        let mut insert=store.db.prepare("INSERT INTO files(root_id,path,name,extension,kind,size,modified,created) VALUES(1,?1,?2,'txt','text',100,0,0)")?;
        for i in 0..count {
            insert.execute(rusqlite::params![
                format!("synthetic/file-{i}.txt"),
                format!("report document {i}.txt")
            ])?;
        }
    }
    store.db.execute_batch("COMMIT")?;
    let metadata_insert_ms = start.elapsed().as_secs_f64() * 1000.;
    let start = Instant::now();
    let matches: i64 = store.db.query_row(
        "SELECT count(*) FROM files_fts WHERE files_fts MATCH 'report'",
        [],
        |r| r.get(0),
    )?;
    let fts_ms = start.elapsed().as_secs_f64() * 1000.;
    let report = serde_json::json!({"synthetic_ann_benchmark":true,"real_model_embeddings":false,"vectors":count,"dimensions":128,"files":count,"build_ms":build_ms,"resident_bytes_after_ann":resident,"ann_p50_ms":samples[50],"ann_p95_ms":samples[95],"save_ms":save_ms,"load_ms":load_ms,"fts_ms":fts_ms,"fts_matches":matches,"metadata_insert_ms":metadata_insert_ms,"hardware":smarti_search_core::Engine::hardware()});
    std::fs::write(
        output.join("benchmark.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
