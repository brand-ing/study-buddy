use crate::data::{Heatmap, SessionConfig, SoundOption, Task};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct SaveData {
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub heatmap: Heatmap,
    #[serde(default)]
    pub next_id: u64,
    #[serde(default)]
    pub pomodoros_done: u32,
    #[serde(default)]
    pub session_config: SessionConfig,
    #[serde(default)]
    pub sound_option: SoundOption,
}

fn data_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("study-buddy")
        .join("data.json")
}

pub fn load() -> SaveData {
    std::fs::read_to_string(data_path())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(d: &SaveData) {
    let path = data_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(d) {
        let _ = std::fs::write(path, json);
    }
}
