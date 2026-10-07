use crate::{
    changes::Changes,
    exclusions::Exclusions,
    extract,
    inference::Inference,
    progress::Progress,
    query,
    store::{Store, now},
    types::*,
    vectors::Vectors,
};
use anyhow::{Context, Result, ensure};
use notify::{RecursiveMode, Watcher};
use rusqlite::params;
use std::{
    collections::{BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant, UNIX_EPOCH},
};

pub type EventSink = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;
struct PendingJob<'a> {
    store: &'a Mutex<Store>,
    id: i64,
}
impl Drop for PendingJob<'_> {
    fn drop(&mut self) {
        // Completed commits remove their job. Interrupted work must become
        // eligible again on resume, without requiring an application restart.
        let _ = self.store.lock().unwrap().db.execute(
            "UPDATE jobs SET state='queued' WHERE file_id=?1 AND state='active'",
            [self.id],
        );
    }
}
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
    pub scan: Mutex<BTreeSet<i64>>,
    scanning: Mutex<BTreeSet<i64>>,
    pub events: EventSink,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
    work: Mutex<()>,
    configuration: RwLock<()>,
    last_emit: Mutex<Instant>,
    changes: Mutex<Changes>,
    watch_rules: RwLock<(Vec<Root>, Arc<Exclusions>)>,
    progress: Mutex<Progress>,
}
impl Engine {
    pub fn open(data: PathBuf, resources: PathBuf, events: EventSink) -> Result<Arc<Self>> {
        std::fs::create_dir_all(data.join("cache"))?;
        let store = Store::open(&data.join("data/metadata.sqlite"))?;
        let settings = store.settings()?;
        let stopped = store
            .db
            .query_row(
                "SELECT value FROM settings WHERE key='index_stopped'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .as_deref()
            == Some("1");
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
            stopped: AtomicBool::new(stopped),
            running: AtomicBool::new(false),
            latest_query: AtomicU64::new(0),
            scan: Mutex::new(BTreeSet::new()),
            scanning: Mutex::new(BTreeSet::new()),
            events,
            watcher: Mutex::new(None),
            work: Mutex::new(()),
            configuration: RwLock::new(()),
            last_emit: Mutex::new(Instant::now() - Duration::from_secs(1)),
            changes: Mutex::new(Changes::default()),
            watch_rules: RwLock::new((
                Vec::new(),
                Arc::new(Exclusions::new(&Settings::default(), &[])),
            )),
            progress: Mutex::new(Progress::default()),
        });
        engine.watch()?;
        // Reconcile changes made while the app was closed. Unchanged readable
        // NTFS journals skip enumeration; other volumes use metadata-only scans.
        let exclusions_changed = engine
            .store
            .lock()
            .unwrap()
            .db
            .query_row(
                "SELECT value FROM settings WHERE key='exclusions_rescan_required'",
                [],
                |r| r.get::<_, String>(0),
            )
            .ok()
            .as_deref()
            == Some("1");
        let roots = engine.store.lock().unwrap().root_definitions()?;
        for root in roots {
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
            if exclusions_changed || journal.is_none() || journal != saved {
                engine.scan.lock().unwrap().insert(root.id);
            }
        }
        engine.status.lock().unwrap().discovery_complete = engine.scan.lock().unwrap().is_empty();
        let weak = Arc::downgrade(&engine);
        thread::spawn(move || {
            let mut was_ready = false;
            let mut previous_health = serde_json::Value::Null;
            let mut last_reconcile = Instant::now();
            loop {
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
                if e.paused.load(Ordering::Relaxed) || e.stopped.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_millis(250));
                    continue;
                }
                let scans: Vec<_> = {
                    let mut pending = e.scan.lock().unwrap();
                    let scans = std::mem::take(&mut *pending);
                    e.scanning.lock().unwrap().extend(&scans);
                    scans.into_iter().collect()
                };
                let paths = e.changes.lock().unwrap().ready();
                let (roots, rules) = e.watch_rules.read().unwrap().clone();
                let mut covered = Vec::<PathBuf>::new();
                for (path, recursive) in paths {
                    if e.stopped.load(Ordering::Relaxed) {
                        e.changes.lock().unwrap().retry(path, recursive);
                        continue;
                    }
                    if roots
                        .iter()
                        .any(|root| scans.contains(&root.id) && path.starts_with(&root.path))
                    {
                        continue;
                    }
                    if covered.iter().any(|parent| path.starts_with(parent)) {
                        continue;
                    }
                    if recursive && path.is_dir() {
                        covered.push(path.clone());
                    }
                    if let Err(error) = e.changed(&path, recursive, &roots, &rules) {
                        let _ = e.store.lock().unwrap().log(
                            "error",
                            None,
                            &display_path(&path),
                            &error.to_string(),
                        );
                    }
                }
                if !scans.is_empty() {
                    if e.stopped.load(Ordering::Relaxed) {
                        e.scan.lock().unwrap().extend(scans);
                        e.scanning.lock().unwrap().clear();
                        continue;
                    }
                    e.running.store(true, Ordering::Relaxed);
                    if let Err(err) = e.begin_discovery(&scans) {
                        let _ = e
                            .store
                            .lock()
                            .unwrap()
                            .log("error", None, "", &err.to_string());
                    }
                    for (position, id) in scans.iter().enumerate() {
                        if e.stopped.load(Ordering::Relaxed) {
                            e.scan
                                .lock()
                                .unwrap()
                                .extend(scans[position..].iter().copied());
                            break;
                        }
                        if let Err(err) = e.discover(*id) {
                            let _ =
                                e.store
                                    .lock()
                                    .unwrap()
                                    .log("error", None, "", &err.to_string());
                        }
                        if e.stopped.load(Ordering::Relaxed) {
                            e.scan
                                .lock()
                                .unwrap()
                                .extend(scans[position..].iter().copied());
                            break;
                        }
                    }
                    last_reconcile = Instant::now();
                    e.scanning.lock().unwrap().clear();
                    {
                        let mut progress = e.progress.lock().unwrap();
                        progress.discovery_total = progress.discovery_done;
                    }
                    let discovery_complete =
                        !e.stopped.load(Ordering::Relaxed) && e.scan.lock().unwrap().is_empty();
                    let mut status = e.status.lock().unwrap();
                    status.discovery_counting = false;
                    status.discovery_complete = discovery_complete;
                    if e.stopped.load(Ordering::Relaxed) {
                        e.running.store(false, Ordering::Relaxed);
                    }
                    drop(status);
                    if !e.stopped.load(Ordering::Relaxed) {
                        let _ = e.store.lock().unwrap().db.execute(
                            "DELETE FROM settings WHERE key='exclusions_rescan_required'",
                            [],
                        );
                    }
                    e.emit_status();
                }
                if e.stopped.load(Ordering::Relaxed) {
                    continue;
                }
                let job = {
                    let s = e.store.lock().unwrap();
                    let media_paused = s.settings().is_ok_and(|settings| settings.pause_on_battery)
                        && crate::platform::power_constrained();
                    s.db.query_row("SELECT file_id,kind FROM jobs INDEXED BY jobs_ready WHERE state='queued' AND online=1 AND (?1=0 OR kind NOT IN ('image','audio','video')) ORDER BY priority,file_id LIMIT 1",[media_paused],|r|Ok((r.get::<_,i64>(0)?, r.get::<_,String>(1)?))).ok()
                };
                if let Some((id, kind)) = job {
                    let started = Instant::now();
                    let paused_before = e.progress.lock().unwrap().paused_seconds();
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
                    if !e.stopped.load(Ordering::Relaxed) && !e.paused.load(Ordering::Relaxed) {
                        let mut progress = e.progress.lock().unwrap();
                        let seconds = (started.elapsed().as_secs_f64()
                            - (progress.paused_seconds() - paused_before))
                            .max(0.);
                        progress.record(&kind, seconds);
                    }
                    e.emit_status();
                } else {
                    if e.running.swap(false, Ordering::Relaxed) {
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
                    // Healthy watchers need no periodic full-root scan. Recover
                    // an overflow once, while idle; also reconnect offline roots.
                    let rescan = std::mem::take(&mut e.changes.lock().unwrap().rescan);
                    if rescan || last_reconcile.elapsed() > Duration::from_secs(300) {
                        let degraded = e.status.lock().unwrap().watcher == "degraded";
                        let roots = e.root_definitions();
                        if let Ok(roots) = roots {
                            let ids: Vec<_> = roots
                                .iter()
                                .filter(|root| {
                                    rescan
                                        || degraded
                                        || root.online != Path::new(&root.path).exists()
                                })
                                .map(|root| root.id)
                                .collect();
                            if !ids.is_empty() {
                                e.queue_scan(ids);
                                let _ = e.watch();
                            }
                        }
                        last_reconcile = Instant::now();
                    }
                    thread::sleep(Duration::from_millis(250));
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
    fn queue_scan(&self, ids: impl IntoIterator<Item = i64>) {
        let mut scan = self.scan.lock().unwrap();
        let scanning = self.scanning.lock().unwrap();
        scan.extend(ids.into_iter().filter(|id| !scanning.contains(id)));
        self.status.lock().unwrap().discovery_complete = false;
    }
    pub fn start_index(&self, root: Option<i64>) -> Result<()> {
        self.running.store(true, Ordering::Relaxed);
        self.stopped.store(false, Ordering::Relaxed);
        self.paused.store(false, Ordering::Relaxed);
        self.progress.lock().unwrap().set_paused(false);
        let ids = if let Some(id) = root {
            vec![id]
        } else {
            self.roots()?.iter().map(|r| r.id).collect()
        };
        self.store.lock().unwrap().db.execute(
            "INSERT OR REPLACE INTO settings VALUES('index_stopped','0')",
            [],
        )?;
        self.queue_scan(ids);
        self.emit_status();
        Ok(())
    }
    pub fn control(&self, action: &str) -> Result<()> {
        self.progress.lock().unwrap().set_paused(action == "pause");
        match action {
            "pause" => self.paused.store(true, Ordering::Relaxed),
            "resume" => {
                self.store.lock().unwrap().db.execute(
                    "INSERT OR REPLACE INTO settings VALUES('index_stopped','0')",
                    [],
                )?;
                self.stopped.store(false, Ordering::Relaxed);
                self.paused.store(false, Ordering::Relaxed)
            }
            "stop" => {
                self.store.lock().unwrap().db.execute(
                    "INSERT OR REPLACE INTO settings VALUES('index_stopped','1')",
                    [],
                )?;
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
            self.store.lock().unwrap().db.execute(
                "INSERT OR REPLACE INTO settings VALUES('index_stopped','0')",
                [],
            )?;
        }
        if old.exclusions != settings.exclusions || old.sensitive_files != settings.sensitive_files
        {
            self.refresh_watch_rules()?;
            // An exclusion change during a scan needs one follow-up pass with
            // the new rules; ordinary repeated rescan requests are coalesced.
            self.scan
                .lock()
                .unwrap()
                .extend(self.root_definitions()?.iter().map(|root| root.id));
            self.status.lock().unwrap().discovery_complete = false;
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
        Exclusions::new(settings, extra).matches(path, root)
    }
    fn root_definitions(&self) -> Result<Vec<Root>> {
        self.store.lock().unwrap().root_definitions()
    }
    fn refresh_watch_rules(&self) -> Result<()> {
        *self.watch_rules.write().unwrap() = (
            self.root_definitions()?,
            Arc::new(Exclusions::new(&self.settings()?, &[])),
        );
        Ok(())
    }
    fn begin_discovery(&self, ids: &[i64]) -> Result<()> {
        // One walk only. Existing metadata gives a provisional denominator;
        // a brand-new location stays "Estimating" until its total is known.
        let previous: u64 = self
            .roots()?
            .iter()
            .filter(|root| ids.contains(&root.id))
            .map(|root| root.files)
            .sum();
        let mut progress = self.progress.lock().unwrap();
        progress.reset_discovery();
        progress.discovery_total = previous;
        progress.start_discovery();
        drop(progress);
        let mut status = self.status.lock().unwrap();
        status.stage = "discovery".into();
        status.discovery_counting = previous == 0;
        status.discovery_complete = false;
        drop(status);
        self.emit_status();
        Ok(())
    }
    fn discover(&self, id: i64) -> Result<()> {
        self.discover_scope(id, None)
    }
    fn discover_scope(&self, id: i64, scope: Option<(&Path, bool)>) -> Result<()> {
        let _work = self.work.lock().unwrap();
        let root = self
            .root_definitions()?
            .into_iter()
            .find(|r| r.id == id)
            .context("Location not found")?;
        let path = PathBuf::from(&root.path);
        let online = path.exists();
        if scope.is_none() {
            let s = self.store.lock().unwrap();
            s.db.execute(
                "UPDATE roots SET online=?2 WHERE id=?1",
                params![id, online],
            )?;
            s.db.execute(
                "UPDATE files SET online=?2 WHERE root_id=?1 AND online<>?2",
                params![id, online],
            )?;
        }
        if !online {
            return Ok(());
        }
        if scope.is_none() {
            self.status.lock().unwrap().stage = "discovery".into();
        }
        let settings = self.settings()?;
        let seen = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let journal = scope
            .is_none()
            .then(|| crate::platform::journal_checkpoint(&path))
            .flatten();
        let mut count = 0;
        let rules = Exclusions::new(&settings, &root.exclusions);
        // Explicit exclusions also remove old index entries when another part
        // of this location is unreadable; an incomplete scan only protects
        // otherwise included files from being mistaken for deleted files.
        let removed = {
            let s = self.store.lock().unwrap();
            let files = Self::scope_files(&s, id, scope)?;
            let tx = s.db.unchecked_transaction()?;
            let mut chunks = Vec::new();
            for (file, filename, _) in files {
                let filename = Path::new(&filename);
                if rules.matches(filename, &path)
                    || filename.starts_with(&self.data)
                    || filename.starts_with(&self.resources)
                {
                    chunks.extend(
                        s.db.prepare("SELECT id FROM chunks WHERE file_id=?1")?
                            .query_map([file], |r| r.get::<_, u64>(0))?
                            .collect::<rusqlite::Result<Vec<_>>>()?,
                    );
                    s.db.execute("DELETE FROM files WHERE id=?1", [file])?;
                }
            }
            tx.commit()?;
            chunks
        };
        {
            let vectors = self.vectors.lock().unwrap();
            for id in removed {
                vectors.remove(id)?;
            }
        }
        let walker = walkdir::WalkDir::new(scope.map_or(path.as_path(), |(path, _)| path))
            .max_depth(if scope.is_some_and(|(_, recursive)| !recursive) {
                1
            } else {
                usize::MAX
            })
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                !rules.matches(e.path(), &path)
                    && !e.path().starts_with(&self.data)
                    && !e.path().starts_with(&self.resources)
            });
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
            if let Err(error) = self.upsert_file(id, entry.path(), seen) {
                errors += 1;
                let _ = self.store.lock().unwrap().log(
                    "skipped",
                    None,
                    &display_path(entry.path()),
                    &error.to_string(),
                );
                continue;
            }
            if scope.is_none() {
                self.progress.lock().unwrap().discovered_file();
            }
            count += 1;
            if count % 128 == 0 {
                let mut status = self.status.lock().unwrap();
                status.discovered = count;
                drop(status);
                self.emit_status();
            }
        }
        // Do not tombstone unseen files when enumeration was incomplete.
        if errors == 0 {
            let removed = {
                let s = self.store.lock().unwrap();
                let files = Self::scope_files(&s, id, scope)?;
                let tx = s.db.unchecked_transaction()?;
                let mut chunks = Vec::<u64>::new();
                for (file, _, _) in files
                    .into_iter()
                    .filter(|(_, _, generation)| *generation < seen)
                {
                    chunks.extend(
                        s.db.prepare("SELECT id FROM chunks WHERE file_id=?1")?
                            .query_map([file], |row| row.get::<_, u64>(0))?
                            .collect::<rusqlite::Result<Vec<_>>>()?,
                    );
                    tx.execute("DELETE FROM files WHERE id=?1", [file])?;
                }
                tx.commit()?;
                chunks
            };
            let vectors = self.vectors.lock().unwrap();
            for chunk in removed {
                vectors.remove(chunk)?;
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
            if scope.is_none() {
                "scan"
            } else {
                "scan_subtree"
            },
            None,
            &display_path(scope.map_or(path.as_path(), |(path, _)| path)),
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
        let s = self.store.lock().unwrap();
        let mut old =
            s.db.query_row(
                "SELECT id,size,mtime_ns,stable_id FROM files WHERE path=?1",
                [&path_text],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, u64>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .ok();
        if old
            .as_ref()
            .is_some_and(|old| old.1 == size && old.2 == mtime_ns)
        {
            s.db.execute("UPDATE files SET seen=CASE WHEN ?2=0 THEN seen ELSE ?2 END,online=1 WHERE id=?1 AND (?2<>0 OR online<>1)", params![old.as_ref().unwrap().0,seen])?;
            return Ok(());
        }
        let tx = s.db.unchecked_transaction()?;
        // A known path needs no native handle or identity lookup. New paths
        // still use the stable NTFS identity to preserve rename semantics.
        let stable = old
            .as_ref()
            .filter(|v| v.1 == size && v.2 == mtime_ns)
            .and_then(|v| v.3.clone())
            .unwrap_or_else(|| {
                crate::file_identity::stable(path).unwrap_or_else(|| path_text.clone())
            });
        let renamed = if old.is_none() {
            s.db.query_row(
                "SELECT id FROM files WHERE stable_id=?1 AND root_id=?2",
                params![stable, root],
                |r| r.get::<_, i64>(0),
            )
            .ok()
        } else {
            None
        };
        if let Some(id) = renamed {
            s.db.execute(
                "UPDATE files SET path=?2,name=?3,content_hash=CASE WHEN kind<>?4 THEN NULL ELSE content_hash END,semantic=CASE WHEN kind<>?4 THEN 0 ELSE semantic END,mtime_ns=CASE WHEN kind<>?4 THEN 0 ELSE mtime_ns END WHERE id=?1",
                params![id, path_text, name,kind],
            )?;
            old =
                s.db.query_row(
                    "SELECT id,size,mtime_ns,stable_id FROM files WHERE id=?1",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .ok();
        }
        s.db.execute("INSERT INTO files(root_id,path,name,extension,kind,size,modified,created,seen,stable_id,mtime_ns) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11) ON CONFLICT(path) DO UPDATE SET size=excluded.size,modified=excluded.modified,mtime_ns=excluded.mtime_ns,stable_id=excluded.stable_id,extension=excluded.extension,kind=excluded.kind,seen=excluded.seen,online=1",params![root,path_text,name,extension,kind,size,modified,created,seen,stable,mtime_ns])?;
        let id = old
            .as_ref()
            .map(|x| x.0)
            .unwrap_or_else(|| s.db.last_insert_rowid());
        if old.is_none_or(|(_, old_size, old_time, _)| old_size != size || old_time != mtime_ns) {
            s.db.execute(
                "INSERT OR REPLACE INTO jobs(file_id,state) VALUES(?1,'queued')",
                [id],
            )?;
        }
        tx.commit()?;
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
        let _pending = PendingJob {
            store: &self.store,
            id,
        };
        let root: (String, String) = self.store.lock().unwrap().db.query_row(
            "SELECT path,exclusions FROM roots WHERE id=?1",
            [file.root_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let root_exclusions: Vec<String> = serde_json::from_str(&root.1)?;
        if Self::excluded(&path, Path::new(&root.0), &settings, &root_exclusions) {
            // A newly excluded queued file must never reach content extraction.
            let ids = {
                let s = self.store.lock().unwrap();
                let ids =
                    s.db.prepare("SELECT id FROM chunks WHERE file_id=?1")?
                        .query_map([id], |r| r.get::<_, u64>(0))?
                        .collect::<rusqlite::Result<Vec<_>>>()?;
                s.db.execute("DELETE FROM files WHERE id=?1", [id])?;
                ids
            };
            let vectors = self.vectors.lock().unwrap();
            for id in ids {
                vectors.remove(id)?;
            }
            return Ok(());
        }
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
            // Changed content with processing disabled must not retain stale
            // searchable chunks (also covers a text -> binary extension move).
            let old = self
                .store
                .lock()
                .unwrap()
                .commit_chunks(id, &[], "", None)?;
            let vectors = self.vectors.lock().unwrap();
            for id in old {
                vectors.remove(id as u64)?;
            }
            return Ok(());
        }
        let snapshot = std::fs::metadata(&path)?;
        self.upsert_file(file.root_id, &path, 0)?;
        ensure!(
            !["text", "code"].contains(&file.kind.as_str())
                || snapshot.len() <= settings.max_text_mb * 1024 * 1024,
            "Text exceeds the configured extraction size limit"
        );
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
            let current = std::fs::metadata(&path)?;
            if current.len() != snapshot.len() || current.modified()? != snapshot.modified()? {
                self.upsert_file(file.root_id, &path, 0)?;
                return Ok(());
            }
            self.store
                .lock()
                .unwrap()
                .db
                .execute("DELETE FROM jobs WHERE file_id=?1", [id])?;
            return Ok(());
        }
        let mut chunks = if ["text", "code"].contains(&file.kind.as_str()) {
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
        let current = std::fs::metadata(&path)?;
        if current.len() != snapshot.len() || current.modified()? != snapshot.modified()? {
            self.upsert_file(file.root_id, &path, 0)?;
            self.cleanup_media(&path)?;
            return Ok(());
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
        self.refresh_watch_rules()?;
        let weak = Arc::downgrade(self);
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let Some(e) = weak.upgrade() else {
                    return;
                };
                match event {
                    Ok(event) => {
                        let (roots, rules) = &*e.watch_rules.read().unwrap();
                        e.changes.lock().unwrap().record(event, |path| {
                            !path.starts_with(&e.data)
                                && !path.starts_with(&e.resources)
                                && roots.iter().any(|root| {
                                    path.starts_with(&root.path)
                                        && !rules.matches(path, Path::new(&root.path))
                                })
                        });
                    }
                    Err(_) => {
                        e.changes.lock().unwrap().rescan = true;
                    }
                }
            })?;
        let mut active = true;
        for root in self.root_definitions()? {
            if Path::new(&root.path).exists()
                && watcher
                    .watch(Path::new(&root.path), RecursiveMode::Recursive)
                    .is_err()
            {
                active = false;
            }
        }
        *self.watcher.lock().unwrap() = Some(watcher);
        self.status.lock().unwrap().watcher = if active { "active" } else { "degraded" }.into();
        Ok(())
    }
    fn scope_files(
        s: &Store,
        root: i64,
        scope: Option<(&Path, bool)>,
    ) -> Result<Vec<(i64, String, i64)>> {
        let read = |row: &rusqlite::Row<'_>| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        };
        if let Some((path, recursive)) = scope {
            let text = display_path(path);
            let separator = std::path::MAIN_SEPARATOR;
            let base = text.trim_end_matches(separator);
            let lower = format!("{base}{separator}");
            let upper = format!("{base}{}", char::from_u32(separator as u32 + 1).unwrap());
            let files =
                s.db.prepare(
                    "SELECT id,path,seen FROM files WHERE root_id=?1 AND path>=?2 AND path<?3",
                )?
                .query_map(params![root, lower, upper], read)?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(files
                .into_iter()
                .filter(|(_, filename, _)| recursive || Path::new(filename).parent() == Some(path))
                .collect())
        } else {
            Ok(s.db
                .prepare("SELECT id,path,seen FROM files WHERE root_id=?1")?
                .query_map([root], read)?
                .collect::<rusqlite::Result<Vec<_>>>()?)
        }
    }
    fn changed(
        &self,
        path: &Path,
        recursive: bool,
        roots: &[Root],
        rules: &Exclusions,
    ) -> Result<()> {
        let Some(root) = roots.iter().find(|root| path.starts_with(&root.path)) else {
            return Ok(());
        };
        if path.starts_with(&self.data)
            || path.starts_with(&self.resources)
            || rules.matches(path, Path::new(&root.path))
        {
            return Ok(());
        }
        if !Path::new(&root.path).try_exists()? {
            let store = self.store.lock().unwrap();
            store
                .db
                .execute("UPDATE roots SET online=0 WHERE id=?1", [root.id])?;
            store.db.execute(
                "UPDATE files SET online=0 WHERE root_id=?1 AND online<>0",
                [root.id],
            )?;
            return Ok(());
        }
        // Inaccessible paths are not deletions. Preserve their cached content.
        if let Err(error) = std::fs::symlink_metadata(path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.into());
        }
        if path.is_file() {
            if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
                return Ok(());
            }
            self.upsert_file(root.id, path, 0)?;
        } else if path.is_dir() {
            // Folder mtime events inspect direct children only. Creates/moves
            // enumerate that subtree once, never the entire indexing location.
            self.discover_scope(root.id, Some((path, recursive)))?;
            if self.stopped.load(Ordering::Relaxed) {
                self.changes
                    .lock()
                    .unwrap()
                    .retry(path.to_owned(), recursive);
            }
        } else {
            let removed = {
                let s = self.store.lock().unwrap();
                let mut files = Self::scope_files(&s, root.id, Some((path, true)))?;
                if let Ok(id) = s.db.query_row(
                    "SELECT id FROM files WHERE path=?1",
                    [display_path(path)],
                    |row| row.get::<_, i64>(0),
                ) {
                    files.push((id, display_path(path), 0));
                }
                let tx = s.db.unchecked_transaction()?;
                let mut chunks = Vec::<u64>::new();
                for (id, _, _) in files {
                    chunks.extend(
                        s.db.prepare("SELECT id FROM chunks WHERE file_id=?1")?
                            .query_map([id], |row| row.get::<_, u64>(0))?
                            .collect::<rusqlite::Result<Vec<_>>>()?,
                    );
                    tx.execute("DELETE FROM files WHERE id=?1", [id])?;
                }
                tx.commit()?;
                chunks
            };
            let vectors = self.vectors.lock().unwrap();
            for id in removed {
                vectors.remove(id)?;
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
        let progress = self.progress.lock().unwrap();
        status.discovery_total = progress.discovery_total;
        status.discovery_processed = progress.discovery_done;
        if progress.discovery_seconds > 0. {
            status.files_per_second = progress.discovery_done as f64 / progress.discovery_seconds;
        }
        status.discovery_eta_seconds = if status.discovery_complete {
            Some(0.)
        } else if status.discovery_counting {
            None
        } else {
            progress.discovery_eta()
        };
        let pending = s.pending_kinds()?;
        let (eta, provisional) = progress.index_eta(&pending);
        status.index_eta_seconds = eta;
        status.index_eta_provisional = provisional;
        drop(progress);
        if !self.running.load(Ordering::Relaxed) || self.paused.load(Ordering::Relaxed) {
            status.discovery_eta_seconds = None;
            status.index_eta_seconds = None;
        }
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
        if last.elapsed() < Duration::from_millis(350) {
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

#[cfg(test)]
mod job_tests {
    use super::*;
    #[test]
    fn stopping_an_active_job_leaves_it_queued_for_resume() {
        let temp = tempfile::tempdir().unwrap();
        let corpus = temp.path().join("corpus");
        let resources = temp.path().join("resources");
        let data = temp.path().join("index");
        std::fs::create_dir(&corpus).unwrap();
        std::fs::create_dir(&resources).unwrap();
        let filename = corpus.join("large.txt");
        std::fs::write(
            &filename,
            "A searchable paragraph about local indexing.\n".repeat(800),
        )
        .unwrap();
        let store = Store::open(&data.join("data/metadata.sqlite")).unwrap();
        store
            .db
            .execute("INSERT INTO settings VALUES('index_stopped','1')", [])
            .unwrap();
        drop(store);
        let engine = Engine::open(data, resources, Arc::new(|_, _| {})).unwrap();
        let root = engine.add_root(&corpus).unwrap();
        let filename = PathBuf::from(display_path(&std::fs::canonicalize(filename).unwrap()));
        engine.paused.store(true, Ordering::Relaxed);
        engine.stopped.store(false, Ordering::Relaxed);
        engine.upsert_file(root, &filename, 1).unwrap();
        let id = engine
            .store
            .lock()
            .unwrap()
            .db
            .query_row("SELECT id FROM files", [], |row| row.get::<_, i64>(0))
            .unwrap();
        let worker = engine.clone();
        let thread = thread::spawn(move || worker.index_file(id));
        let started = Instant::now();
        loop {
            let state: String = engine
                .store
                .lock()
                .unwrap()
                .db
                .query_row("SELECT state FROM jobs WHERE file_id=?1", [id], |row| {
                    row.get(0)
                })
                .unwrap();
            if state == "active" {
                break;
            }
            assert!(started.elapsed() < Duration::from_secs(10));
            thread::sleep(Duration::from_millis(10));
        }
        engine.control("stop").unwrap();
        thread.join().unwrap().unwrap();
        let state: String = engine
            .store
            .lock()
            .unwrap()
            .db
            .query_row("SELECT state FROM jobs WHERE file_id=?1", [id], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(state, "queued");
        assert_eq!(engine.status().unwrap().semantic_files, 0);
    }
}
