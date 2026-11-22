use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const TASK_LIMIT: usize = 120;
pub const DEFAULT_INTERVAL: u32 = 30;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub task: String,
    /// Wall-clock time the current task was set, so the nudge clock survives restarts.
    pub task_set_at_ms: i64,
    /// 0 = reminders off.
    pub interval_minutes: u32,
    pub pos: Option<SavedPos>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPos {
    pub monitor: Option<String>,
    /// Physical offset from the monitor origin.
    pub dx: i32,
    pub dy: i32,
}

impl Default for Saved {
    fn default() -> Self {
        Self {
            task: String::new(),
            task_set_at_ms: 0,
            interval_minutes: DEFAULT_INTERVAL,
            pos: None,
        }
    }
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn load(dir: &Path) -> Saved {
    std::fs::read(dir.join("state.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save(dir: &Path, data: &Saved) {
    if let Err(err) = try_save(dir, data) {
        eprintln!("nudge: could not save state: {err}");
    }
}

fn try_save(dir: &Path, data: &Saved) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    let json = serde_json::to_vec_pretty(data).map_err(std::io::Error::other)?;
    let tmp = dir.join("state.json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, dir.join("state.json"))
}

/// Trim, drop control characters and cap the length so the pill stays a pill.
pub fn sanitize(raw: &str) -> String {
    let cleaned: String = raw.chars().filter(|c| !c.is_control()).collect();
    cleaned
        .trim()
        .chars()
        .take(TASK_LIMIT)
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn valid_interval(minutes: u32) -> u32 {
    match minutes {
        0 | 15 | 30 | 45 | 60 => minutes,
        _ => DEFAULT_INTERVAL,
    }
}
