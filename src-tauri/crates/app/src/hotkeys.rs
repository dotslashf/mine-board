use std::collections::HashMap;

use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::state::AppState;

/// Normalize a shortcut string from clip settings (`ctrl+shift+1`, `f5`,
/// `MediaPlayPause`) into a canonical accelerator string. Both sides of the
/// shortcut path (registration and dispatch) run through this function so the
/// map keys always agree.
pub fn normalize_shortcut(input: &str) -> String {
    let mut mods: Vec<String> = Vec::new();
    let mut key: Option<String> = None;
    for part in input.split('+') {
        let p = part.trim();
        match p.to_lowercase().as_str() {
            "ctrl" | "control" | "commandorcontrol" | "cmdorctrl" => mods.push("Control".into()),
            "alt" | "option" => mods.push("Alt".into()),
            "shift" => mods.push("Shift".into()),
            "super" | "meta" | "cmd" | "win" | "command" | "cmdorwin" => {
                mods.push("Super".into())
            }
            "" => {}
            other => key = Some(other.to_lowercase()),
        }
    }
    mods.push(key.unwrap_or_default());
    mods.join("+")
}

/// Rebuild all registered global shortcuts from the current clips + settings.
/// Unregisters everything first, so changes are always idempotent.
pub fn sync_hotkeys(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister_all();

    let settings = atcs_soundboard_core::settings::load_settings(&state.db.lock().unwrap())
        .map_err(|e| e.to_string())?;
    if !settings.soundboard.global_hotkeys {
        state.hotkey_map.lock().unwrap().clear();
        return Ok(());
    }

    let clips = state
        .db
        .lock()
        .unwrap()
        .list_clips(None)
        .map_err(|e| e.to_string())?;

    let mut map: HashMap<String, i64> = HashMap::new();
    for clip in clips {
        if !clip.enabled {
            continue;
        }
        let Some(shortcut_str) = clip.shortcut else {
            continue;
        };
        let normalized = normalize_shortcut(&shortcut_str);
        let Ok(shortcut) = Shortcut::try_from(normalized.as_str()) else {
            continue;
        };
        if shortcuts.register(shortcut).is_ok() {
            map.insert(normalized, clip.id);
        }
    }
    *state.hotkey_map.lock().unwrap() = map;
    Ok(())
}
