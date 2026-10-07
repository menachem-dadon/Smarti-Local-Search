use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,
    pub theme: String,
    pub reduced_motion: bool,
    pub text_size: u16,
    pub onboarded: bool,
    pub default_mode: String,
    pub results_count: usize,
    pub search_as_you_type: bool,
    pub enter_preview: bool,
    pub show_offline: bool,
    pub profile: String,
    pub accelerator: String,
    pub threads: usize,
    pub concurrency: usize,
    pub dimensions: usize,
    pub vision_tokens: usize,
    pub text: bool,
    pub code: bool,
    pub documents: bool,
    pub images: bool,
    pub audio: bool,
    pub video: bool,
    pub audio_seconds: f64,
    pub video_seconds: f64,
    pub video_fps: f64,
    pub pause_on_battery: bool,
    pub network_drives: bool,
    pub sensitive_files: bool,
    pub exclusions: Vec<String>,
    pub max_text_mb: u64,
    pub cache_mb: u64,
    pub keep_ready: bool,
    pub minimize_to_tray: bool,
    pub autostart: bool,
    pub quick_search: bool,
    pub shortcut: String,
    pub explorer_menu: bool,
    pub notifications: bool,
    pub debug_logging: bool,
    pub index_path: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "he".into(),
            theme: "system".into(),
            reduced_motion: false,
            text_size: 16,
            onboarded: false,
            default_mode: "smart".into(),
            results_count: 100,
            search_as_you_type: true,
            enter_preview: false,
            show_offline: false,
            profile: "auto".into(),
            accelerator: "auto".into(),
            threads: 4,
            concurrency: 1,
            dimensions: 256,
            vision_tokens: 70,
            text: true,
            code: true,
            documents: true,
            images: true,
            audio: true,
            video: true,
            audio_seconds: 60.,
            video_seconds: 30.,
            video_fps: 1.,
            pause_on_battery: true,
            network_drives: false,
            sensitive_files: false,
            exclusions: [
                "node_modules",
                "target",
                "dist",
                "build",
                ".venv",
                "venv",
                "__pycache__",
                "$Recycle.Bin",
                "System Volume Information",
                "AppData",
                ".git/objects",
                ".next",
                ".idea",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            max_text_mb: 32,
            cache_mb: 512,
            keep_ready: true,
            minimize_to_tray: true,
            autostart: false,
            quick_search: true,
            shortcut: "Ctrl+Alt+Space".into(),
            explorer_menu: false,
            notifications: true,
            debug_logging: false,
            index_path: String::new(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            [128, 256, 512, 768].contains(&self.dimensions),
            "Unsupported vector dimensions"
        );
        anyhow::ensure!(
            (1..=500).contains(&self.results_count),
            "Results must be 1..500"
        );
        anyhow::ensure!(
            (1..=128).contains(&self.threads) && (1..=8).contains(&self.concurrency),
            "Invalid worker count"
        );
        anyhow::ensure!(
            (10.0..=120.0).contains(&self.audio_seconds)
                && (10.0..=60.0).contains(&self.video_seconds),
            "Invalid media segment duration"
        );
        anyhow::ensure!((0.1..=4.0).contains(&self.video_fps), "Invalid video FPS");
        anyhow::ensure!((13..=24).contains(&self.text_size), "Invalid text size");
        anyhow::ensure!(
            ["he", "en"].contains(&self.language.as_str()),
            "Invalid language"
        );
        anyhow::ensure!(
            ["system", "light", "dark"].contains(&self.theme.as_str()),
            "Invalid theme"
        );
        anyhow::ensure!(
            ["smart", "semantic", "exact"].contains(&self.default_mode.as_str()),
            "Invalid search mode"
        );
        anyhow::ensure!(
            ["auto", "quiet", "balanced", "fast"].contains(&self.profile.as_str()),
            "Invalid performance profile"
        );
        anyhow::ensure!(
            ["auto", "cpu", "gpu", "npu"].contains(&self.accelerator.as_str()),
            "Invalid accelerator"
        );
        anyhow::ensure!(
            (1..=512).contains(&self.max_text_mb) && (32..=8192).contains(&self.cache_mb),
            "Invalid storage limits"
        );
        Ok(())
    }
    pub fn fingerprint(&self) -> String {
        blake3::hash(
            format!(
                "e7a8a220|{}|{}|search-result/code-retrieval|chunk-v1|schema-1|{}|{}|{}",
                self.dimensions,
                self.vision_tokens,
                self.audio_seconds,
                self.video_seconds,
                self.video_fps
            )
            .as_bytes(),
        )
        .to_hex()
        .to_string()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Root {
    pub id: i64,
    pub path: String,
    pub online: bool,
    pub files: u64,
    pub indexed: u64,
    pub exclusions: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Chunk {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub modality: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub heading: String,
    pub page: Option<u32>,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
    pub start: Option<f64>,
    pub end: Option<f64>,
    pub media_path: Option<String>,
    #[serde(default)]
    pub frames: Vec<String>,
    #[serde(default)]
    pub hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileRecord {
    pub id: i64,
    pub root_id: i64,
    pub path: String,
    pub name: String,
    pub extension: String,
    pub kind: String,
    pub size: u64,
    pub modified: i64,
    pub created: i64,
    pub state: String,
    pub semantic: bool,
    pub error: Option<String>,
    pub online: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Filters {
    pub kind: Option<String>,
    pub extension: Option<String>,
    pub path: Option<String>,
    pub name: Option<String>,
    pub after: Option<i64>,
    pub before: Option<i64>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchRequest {
    pub id: u64,
    pub query: String,
    pub mode: String,
    #[serde(default)]
    pub filters: Filters,
    #[serde(default)]
    pub media: Option<String>,
    #[serde(default)]
    pub semantic_pass: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub file: FileRecord,
    pub matches: Vec<Chunk>,
    pub score: f64,
    pub lexical: f64,
    pub semantic: f64,
    pub filename: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    pub id: u64,
    pub results: Vec<SearchResult>,
    pub elapsed_ms: f64,
    pub semantic_ready: bool,
    pub warning: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IndexStatus {
    pub stage: String,
    pub running: bool,
    pub paused: bool,
    pub discovered: u64,
    pub files: u64,
    pub semantic_files: u64,
    pub chunks: u64,
    pub vectors: u64,
    pub errors: u64,
    pub pending: u64,
    pub bytes: u64,
    pub files_per_second: f64,
    pub embeddings_per_second: f64,
    pub updated: i64,
    pub inference: serde_json::Value,
    pub watcher: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Activity {
    pub id: i64,
    pub file_id: Option<i64>,
    pub time: i64,
    pub kind: String,
    pub path: String,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preview {
    pub file: FileRecord,
    pub chunks: Vec<Chunk>,
    pub text: String,
    pub asset: Option<String>,
}
