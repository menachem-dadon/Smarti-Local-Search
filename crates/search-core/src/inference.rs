use crate::types::Settings;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    collections::BinaryHeap,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};

#[derive(Serialize)]
struct Request<'a> {
    id: u64,
    operation: &'a str,
    arguments: serde_json::Value,
}
#[derive(Deserialize)]
pub struct Response {
    pub id: u64,
    pub status: String,
    #[serde(default)]
    pub data: serde_json::Value,
    #[serde(default, with = "serde_bytes")]
    pub binary: Vec<u8>,
    #[serde(default)]
    pub error: Option<String>,
}
struct Process {
    child: Child,
    input: ChildStdin,
    output: ChildStdout,
    id: u64,
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Process {
    fn spawn(resources: &Path, cache: &Path) -> Result<Self> {
        let exe = resources.join("inference/smarti-local-search-inference.exe");
        let mut command = if exe.exists() {
            Command::new(exe)
        } else if cfg!(debug_assertions) {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
            let python = root.join(".venv/Scripts/python.exe");
            let mut cmd = Command::new(python);
            cmd.arg(root.join("inference-host/src/host.py"));
            cmd
        } else {
            bail!("Bundled inference host is missing. Repair the installation.")
        };
        command
            .args([
                "--resources",
                resources.to_str().context("Resource path")?,
                "--cache",
                cache.to_str().context("Cache path")?,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000 | 0x00004000);
        }
        let mut child = command
            .spawn()
            .context("Starting bundled inference worker")?;
        Ok(Self {
            input: child.stdin.take().unwrap(),
            output: child.stdout.take().unwrap(),
            child,
            id: 0,
        })
    }
    fn call(&mut self, op: &str, args: serde_json::Value) -> Result<Response> {
        let _deadline =
            crate::platform::deadline(self.child.id(), if op == "benchmark" { 290 } else { 120 });
        self.id += 1;
        let payload = rmp_serde::to_vec_named(&Request {
            id: self.id,
            operation: op,
            arguments: args,
        })?;
        self.input
            .write_all(&(payload.len() as u32).to_le_bytes())?;
        self.input.write_all(&payload)?;
        self.input.flush()?;
        let mut prefix = [0; 4];
        self.output.read_exact(&mut prefix)?;
        let len = u32::from_le_bytes(prefix) as usize;
        anyhow::ensure!(
            len > 0 && len <= 64 * 1024 * 1024,
            "Invalid inference frame length"
        );
        let mut data = vec![0; len];
        self.output.read_exact(&mut data)?;
        let response: Response = rmp_serde::from_slice(&data)?;
        anyhow::ensure!(response.id == self.id, "Inference response ID mismatch");
        if response.status != "ok" {
            bail!("{}", response.error.as_deref().unwrap_or("Inference error"))
        }
        Ok(response)
    }
}
struct Job {
    priority: u8,
    sequence: u64,
    operation: String,
    args: serde_json::Value,
    reply: mpsc::Sender<Result<Response>>,
}
impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.sequence == other.sequence
    }
}
impl Eq for Job {}
impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}
#[derive(Clone)]
pub struct Inference {
    sender: mpsc::Sender<Job>,
    sequence: Arc<std::sync::atomic::AtomicU64>,
    pub health: Arc<Mutex<serde_json::Value>>,
}
impl Inference {
    pub fn start(resources: PathBuf, cache: PathBuf, settings: Settings) -> Self {
        let (sender, receiver) = mpsc::channel::<Job>();
        let health = Arc::new(Mutex::new(
            serde_json::json!({"ready":false,"state":"loading"}),
        ));
        let state = health.clone();
        thread::spawn(move || {
            let mut config = settings;
            let mut process = match Process::spawn(&resources, &cache) {
                Ok(process) => Some(process),
                Err(error) => {
                    *state.lock().unwrap() = serde_json::json!({"ready":false,"state":"error","error":error.to_string()});
                    None
                }
            };
            if let Some(p) = process.as_mut() {
                match p.call("initialize", serde_json::to_value(&config).unwrap()) {
                    Ok(r) => *state.lock().unwrap() = r.data,
                    Err(e) => {
                        *state.lock().unwrap() =
                            serde_json::json!({"ready":false,"error":e.to_string()})
                    }
                }
            }
            if !state.lock().unwrap()["ready"].as_bool().unwrap_or(false)
                && config.accelerator != "cpu"
            {
                process = None;
                config.accelerator = "cpu".into();
                if let Ok(mut p) = Process::spawn(&resources, &cache) {
                    match p.call("initialize", serde_json::to_value(&config).unwrap()) {
                        Ok(r) => {
                            let mut health = r.data;
                            health["fallback"] = serde_json::json!("cpu");
                            *state.lock().unwrap() = health;
                        }
                        Err(error) => {
                            *state.lock().unwrap() =
                                serde_json::json!({"ready":false,"error":error.to_string()})
                        }
                    }
                    process = Some(p);
                }
            }
            let mut queue = BinaryHeap::new();
            let mut last_used = std::time::Instant::now();
            let mut idle = false;
            loop {
                if queue.is_empty() {
                    match receiver.recv_timeout(Duration::from_secs(30)) {
                        Ok(j) => queue.push(j),
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if !config.keep_ready
                                && !idle
                                && last_used.elapsed() > Duration::from_secs(120)
                            {
                                if let Some(p) = process.as_mut() {
                                    let _ = p.call("shutdown", serde_json::json!({}));
                                }
                                *state.lock().unwrap() = serde_json::json!({"ready":false,"state":"idle","backend":"unloaded"});
                                idle = true;
                            }
                            continue;
                        }
                    }
                }
                while let Ok(job) = receiver.try_recv() {
                    queue.push(job)
                }
                let Some(job) = queue.pop() else { continue };
                if idle
                    && ["embed", "benchmark"].contains(&job.operation.as_str())
                    && let Some(p) = process.as_mut()
                    && let Ok(r) = p.call("initialize", serde_json::to_value(&config).unwrap())
                {
                    *state.lock().unwrap() = r.data;
                    idle = false;
                }
                last_used = std::time::Instant::now();
                if ["initialize", "configure"].contains(&job.operation.as_str())
                    && let Ok(s) = serde_json::from_value(job.args.clone())
                {
                    config = s;
                }
                let mut result = if let Some(p) = process.as_mut() {
                    p.call(&job.operation, job.args.clone())
                } else {
                    Err(anyhow::anyhow!("Inference worker unavailable"))
                };
                // Only restart on a dead process, once per request. Initialization falls back to CPU.
                let crashed = process
                    .as_mut()
                    .is_none_or(|p| p.child.try_wait().ok().flatten().is_some());
                if crashed
                    || result.is_err() && job.operation == "embed" && config.accelerator != "cpu"
                {
                    process = None;
                    config.accelerator = "cpu".into();
                    if let Ok(mut p) = Process::spawn(&resources, &cache) {
                        if let Ok(r) = p.call("initialize", serde_json::to_value(&config).unwrap())
                        {
                            *state.lock().unwrap() = r.data;
                            result = p.call(&job.operation, job.args.clone());
                        }
                        process = Some(p);
                    }
                }
                if let Ok(ref r) = result
                    && ["initialize", "configure", "health", "benchmark"]
                        .contains(&job.operation.as_str())
                {
                    *state.lock().unwrap() = r.data.clone();
                    if job.operation == "initialize" {
                        idle = false;
                    }
                }
                let _ = job.reply.send(result);
            }
        });
        Self {
            sender,
            sequence: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            health,
        }
    }
    pub fn call(&self, op: &str, args: serde_json::Value, priority: u8) -> Result<Response> {
        let (reply, rx) = mpsc::channel();
        let sequence = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.sender
            .send(Job {
                priority,
                sequence,
                operation: op.into(),
                args,
                reply,
            })
            .map_err(|_| anyhow::anyhow!("Inference scheduler stopped"))?;
        rx.recv_timeout(Duration::from_secs(300))
            .context("Inference timed out")?
    }
    pub fn ready(&self) -> bool {
        self.health.lock().unwrap()["ready"]
            .as_bool()
            .unwrap_or(false)
    }
    pub fn embed(
        &self,
        items: serde_json::Value,
        priority: u8,
        dim: usize,
    ) -> Result<Vec<Vec<f32>>> {
        let r = self.call("embed", serde_json::json!({"items":items}), priority)?;
        anyhow::ensure!(
            r.binary.len().is_multiple_of(dim * 4),
            "Invalid embedding frame"
        );
        Ok(r.binary
            .chunks_exact(dim * 4)
            .map(|row| {
                row.as_chunks::<4>()
                    .0
                    .iter()
                    .map(|v| f32::from_le_bytes(*v))
                    .collect()
            })
            .collect())
    }
}
