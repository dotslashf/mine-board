use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use atcs_soundboard_core::db::Repository;

use crate::engine::Engine;

/// Decoded clip audio kept in memory so retriggering is instant.
#[derive(Clone)]
pub struct CachedClip {
    pub samples: Arc<Vec<f32>>,
    pub frames: usize,
    pub volume: f32,
    pub file_path: String,
    pub modified: Option<std::time::SystemTime>,
}

pub struct AppState {
    pub db: Mutex<Repository>,
    pub engine: Mutex<Option<Engine>>,
    pub clip_cache: Mutex<HashMap<i64, CachedClip>>,
    /// normalized shortcut string -> clip id
    pub hotkey_map: Mutex<HashMap<String, i64>>,
    pub app: tauri::AppHandle,
}
