use std::path::{Path, PathBuf};

use atcs_soundboard_core::audio::wav::decode_wav_file;
use atcs_soundboard_core::db::Repository;
use atcs_soundboard_core::errors::Error;
use atcs_soundboard_core::models::{
    AppSettings, AudioStatus, Category, MicStatus, NewCategory, NewSoundClip, SoundClip,
    UpdateSoundClip, VirtualMicStatus,
};
use atcs_soundboard_core::settings as settings_core;
use tauri::{AppHandle, State};

use crate::engine::EngineCommand;
use crate::hotkeys::sync_hotkeys;
use crate::state::{AppState, CachedClip};

fn err_string(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn repo(state: &AppState) -> std::sync::MutexGuard<'_, Repository> {
    state.db.lock().unwrap()
}

fn sanitize_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect();
    cleaned.trim_matches('.').chars().take(60).collect::<String>()
}

fn import_to_library(source_path: &str) -> Result<PathBuf, String> {
    let src = Path::new(source_path);
    if !src.is_file() {
        return Err(format!("source file not found: {source_path}"));
    }
    let dir = Error::ensure_data_dir()
        .map_err(err_string)?
        .join("clips");
    std::fs::create_dir_all(&dir).map_err(err_string)?;
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .map(sanitize_file_name)
        .unwrap_or_else(|| "clip".into());
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("wav")
        .to_lowercase();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let dest = dir.join(format!("{timestamp}-{stem}.{ext}"));
    std::fs::copy(src, &dest).map_err(err_string)?;
    // Fail fast on formats we cannot play. Only WAV is supported today; the
    // UI filters the picker, but a manual path or renamed file must not
    // create a clip that can never play.
    if let Err(e) = decode_wav_file(&dest) {
        let _ = std::fs::remove_file(&dest);
        return Err(format!(
            "unsupported audio file: {e} (only WAV is supported)"
        ));
    }
    Ok(dest)
}

fn duration_of(path: &str) -> i64 {
    decode_wav_file(path).map(|d| d.duration_ms).unwrap_or(0)
}

// ---------- Audio engine ----------

#[tauri::command]
pub fn start_audio_engine(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut engine = state.engine.lock().unwrap();
    if engine.is_some() {
        return Ok(());
    }
    let db = repo(&state);
    let eng = crate::engine::Engine::start(app.clone(), &db).map_err(err_string)?;
    drop(db);
    *engine = Some(eng);
    sync_hotkeys(&app, &state).map_err(err_string)?;
    Ok(())
}

#[tauri::command]
pub fn stop_audio_engine(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(mut eng) = state.engine.lock().unwrap().take() {
        eng.stop();
    }
    Ok(())
}

#[tauri::command]
pub fn get_audio_status(state: State<'_, AppState>) -> Result<AudioStatus, String> {
    let engine = state.engine.lock().unwrap();
    match engine.as_ref() {
        Some(eng) => Ok(eng.status()),
        None => Ok(AudioStatus {
            engine_running: false,
            microphone: MicStatus::default(),
            virtual_microphone: VirtualMicStatus::default(),
            sample_rate: 48_000,
            channels: 2,
            master_volume: 0.9,
            mic_volume: 1.0,
            soundboard_volume: 1.0,
            monitor_enabled: false,
        }),
    }
}

#[tauri::command]
pub fn get_virtual_microphone_status(state: State<'_, AppState>) -> Result<VirtualMicStatus, String> {
    let engine = state.engine.lock().unwrap();
    Ok(engine
        .as_ref()
        .map(|e| e.status().virtual_microphone)
        .unwrap_or_default())
}

#[tauri::command]
pub fn get_microphone_status(state: State<'_, AppState>) -> Result<MicStatus, String> {
    let engine = state.engine.lock().unwrap();
    Ok(engine
        .as_ref()
        .map(|e| e.status().microphone)
        .unwrap_or_default())
}

#[tauri::command]
pub fn get_audio_devices(state: State<'_, AppState>) -> Result<Vec<atcs_soundboard_core::models::AudioDevice>, String> {
    let engine = state.engine.lock().unwrap();
    Ok(engine.as_ref().map(|e| e.devices()).unwrap_or_default())
}

#[tauri::command]
pub fn get_input_devices(state: State<'_, AppState>) -> Result<Vec<atcs_soundboard_core::models::AudioDevice>, String> {
    let engine = state.engine.lock().unwrap();
    Ok(engine.as_ref().map(|e| e.input_devices()).unwrap_or_default())
}

fn persist_audio_key(db: &Repository, key: &str, value: &str) {
    let _ = db.set_setting(key, value);
}

#[tauri::command]
pub fn select_microphone(_app: AppHandle, state: State<'_, AppState>, device_id: String) -> Result<(), String> {
    let id: Option<u32> = if device_id.is_empty() {
        None
    } else {
        match device_id.parse::<u32>() {
            Ok(id) => Some(id),
            Err(_) => return Err(format!("invalid device id: {device_id}")),
        }
    };
    let db = repo(&state);
    let persist = match id {
        Some(id) => id.to_string(),
        None => String::new(),
    };
    persist_audio_key(&db, "audio.microphone", &persist);
    drop(db);

    let engine = state.engine.lock().unwrap();
    match engine.as_ref() {
        Some(eng) => {
            eng.send(EngineCommand::SelectMicrophone(id));
            Ok(())
        }
        None => Err("Audio engine is not running".into()),
    }
}

#[tauri::command]
pub fn set_mic_volume(state: State<'_, AppState>, volume: f64) -> Result<(), String> {
    let volume = volume.clamp(0.0, 2.0);
    persist_audio_key(&repo(&state), "audio.microphone_volume", &volume.to_string());
    if let Some(eng) = state.engine.lock().unwrap().as_ref() {
        eng.send(EngineCommand::SetMicVolume(volume as f32));
    }
    Ok(())
}

#[tauri::command]
pub fn set_soundboard_volume(state: State<'_, AppState>, volume: f64) -> Result<(), String> {
    let volume = volume.clamp(0.0, 2.0);
    persist_audio_key(&repo(&state), "audio.soundboard_volume", &volume.to_string());
    if let Some(eng) = state.engine.lock().unwrap().as_ref() {
        eng.send(EngineCommand::SetSoundboardVolume(volume as f32));
    }
    Ok(())
}

#[tauri::command]
pub fn set_master_volume(state: State<'_, AppState>, volume: f64) -> Result<(), String> {
    let volume = volume.clamp(0.0, 2.0);
    persist_audio_key(&repo(&state), "audio.master_volume", &volume.to_string());
    if let Some(eng) = state.engine.lock().unwrap().as_ref() {
        eng.send(EngineCommand::SetMasterVolume(volume as f32));
    }
    Ok(())
}

#[tauri::command]
pub fn set_mic_muted(state: State<'_, AppState>, muted: bool) -> Result<(), String> {
    if let Some(eng) = state.engine.lock().unwrap().as_ref() {
        eng.send(EngineCommand::SetMicMuted(muted));
    }
    Ok(())
}

#[tauri::command]
pub fn set_monitor_enabled(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    persist_audio_key(&repo(&state), "audio.monitor_enabled", &enabled.to_string());
    if let Some(eng) = state.engine.lock().unwrap().as_ref() {
        eng.send(EngineCommand::SetMonitorEnabled(enabled));
    }
    Ok(())
}

// ---------- Playback ----------

fn resolve_clip_audio(state: &AppState, clip: &SoundClip) -> Result<CachedClip, String> {
    let modified = std::fs::metadata(&clip.file_path)
        .and_then(|m| m.modified())
        .ok();
    let mut cache = state.clip_cache.lock().unwrap();
    if let Some(c) = cache.get(&clip.id) {
        if c.file_path == clip.file_path && c.modified == modified {
            return Ok(c.clone());
        }
    }
    let decoded = decode_wav_file(&clip.file_path).map_err(err_string)?;
    let entry = CachedClip {
        samples: decoded.samples,
        frames: decoded.frames,
        volume: clip.volume as f32,
        file_path: clip.file_path.clone(),
        modified,
    };
    cache.insert(clip.id, entry.clone());
    Ok(entry)
}

pub fn play_clip_internal(_app: &AppHandle, state: &AppState, clip_id: i64) -> Result<(), String> {
    let clip = repo(state).get_clip(clip_id).map_err(err_string)?;
    if !clip.enabled {
        return Ok(());
    }
    let cached = resolve_clip_audio(state, &clip)?;
    let engine = state.engine.lock().unwrap();
    let eng = engine
        .as_ref()
        .ok_or_else(|| "Audio engine is not running".to_string())?;
    eng.send(EngineCommand::PlayClip {
        clip_id,
        name: clip.name,
        samples: cached.samples.clone(),
        frames: cached.frames,
        gain: cached.volume,
    });
    Ok(())
}

#[tauri::command]
pub fn play_sound(app: AppHandle, state: State<'_, AppState>, clip_id: i64) -> Result<(), String> {
    play_clip_internal(&app, &state, clip_id)
}

#[tauri::command]
pub fn stop_sound(state: State<'_, AppState>, clip_id: i64) -> Result<(), String> {
    if let Some(eng) = state.engine.lock().unwrap().as_ref() {
        eng.send(EngineCommand::StopClip(clip_id));
    }
    Ok(())
}

#[tauri::command]
pub fn stop_all_sounds(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(eng) = state.engine.lock().unwrap().as_ref() {
        eng.send(EngineCommand::StopAll);
    }
    Ok(())
}

// ---------- Clips ----------

#[tauri::command]
pub fn list_sound_clips(state: State<'_, AppState>) -> Result<Vec<SoundClip>, String> {
    repo(&state).list_clips(None).map_err(err_string)
}

#[tauri::command]
pub fn create_sound_clip(
    app: AppHandle,
    state: State<'_, AppState>,
    clip: NewSoundClip,
) -> Result<SoundClip, String> {
    let mut clip = clip;
    if clip.duration_ms == 0 {
        clip.duration_ms = duration_of(&clip.file_path);
    }
    let created = repo(&state).create_clip(clip).map_err(err_string)?;
    sync_hotkeys(&app, &state).map_err(err_string)?;
    Ok(created)
}

#[tauri::command]
pub fn update_sound_clip(
    app: AppHandle,
    state: State<'_, AppState>,
    clip: UpdateSoundClip,
) -> Result<SoundClip, String> {
    let id = clip.id;
    let updated = repo(&state).update_clip(id, clip).map_err(err_string)?;
    state.clip_cache.lock().unwrap().remove(&id);
    sync_hotkeys(&app, &state).map_err(err_string)?;
    Ok(updated)
}

#[tauri::command]
pub fn delete_sound_clip(app: AppHandle, state: State<'_, AppState>, clip_id: i64) -> Result<(), String> {
    {
        let db = repo(&state);
        let clip = db.get_clip(clip_id).ok();
        db.delete_clip(clip_id).map_err(err_string)?;
        if let Some(c) = clip {
            let _ = std::fs::remove_file(&c.file_path);
        }
    }
    state.clip_cache.lock().unwrap().remove(&clip_id);
    sync_hotkeys(&app, &state).map_err(err_string)?;
    Ok(())
}

#[tauri::command]
pub fn import_audio_file(source_path: String) -> Result<String, String> {
    import_to_library(&source_path)
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn import_sound_clip(
    app: AppHandle,
    state: State<'_, AppState>,
    source_path: String,
    name: Option<String>,
) -> Result<SoundClip, String> {
    let dest = import_to_library(&source_path)?;
    let dest_str = dest.to_string_lossy().into_owned();
    let suggested = Path::new(&source_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Clip")
        .to_string();
    let clip = NewSoundClip {
        name: name.unwrap_or(suggested),
        file_path: dest_str,
        ..Default::default()
    };
    create_sound_clip(app, state, clip)
}

#[tauri::command]
pub fn replace_sound_clip_audio(
    app: AppHandle,
    state: State<'_, AppState>,
    clip_id: i64,
    source_path: String,
) -> Result<SoundClip, String> {
    let dest = import_to_library(&source_path)?;
    let dest_str = dest.to_string_lossy().into_owned();
    let old = repo(&state).get_clip(clip_id).ok();
    let duration_ms = duration_of(&dest_str);
    let updated = repo(&state)
        .update_clip(
            clip_id,
            UpdateSoundClip {
                id: clip_id,
                file_path: Some(dest_str),
                duration_ms: Some(duration_ms),
                ..Default::default()
            },
        )
        .map_err(err_string)?;
    if let Some(old) = old {
        if old.file_path != updated.file_path {
            let _ = std::fs::remove_file(&old.file_path);
        }
    }
    state.clip_cache.lock().unwrap().remove(&clip_id);
    sync_hotkeys(&app, &state).map_err(err_string)?;
    Ok(updated)
}

// ---------- Categories ----------

#[tauri::command]
pub fn list_categories(state: State<'_, AppState>) -> Result<Vec<Category>, String> {
    repo(&state).list_categories().map_err(err_string)
}

#[tauri::command]
pub fn create_category(state: State<'_, AppState>, category: NewCategory) -> Result<Category, String> {
    repo(&state).create_category(category).map_err(err_string)
}

#[tauri::command]
pub fn update_category(state: State<'_, AppState>, id: i64, name: String) -> Result<Category, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("name must not be empty".into());
    }
    repo(&state).update_category_name(id, &name).map_err(err_string)
}

#[tauri::command]
pub fn delete_category(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    repo(&state).delete_category(id).map_err(err_string)
}

// ---------- Settings ----------

#[tauri::command]
pub fn get_app_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    settings_core::load_settings(&repo(&state)).map_err(err_string)
}

#[tauri::command]
pub fn set_setting(state: State<'_, AppState>, key: String, value: String) -> Result<(), String> {
    repo(&state).set_setting(&key, &value).map_err(err_string)?;
    match key.as_str() {
        "audio.microphone_volume" => {
            if let Ok(v) = value.parse::<f64>() {
                let _ = set_mic_volume(state, v);
            }
        }
        "audio.soundboard_volume" => {
            if let Ok(v) = value.parse::<f64>() {
                let _ = set_soundboard_volume(state, v);
            }
        }
        "audio.master_volume" => {
            if let Ok(v) = value.parse::<f64>() {
                let _ = set_master_volume(state, v);
            }
        }
        "audio.monitor_enabled" => {
            if let Ok(v) = value.parse::<bool>() {
                let _ = set_monitor_enabled(state, v);
            }
        }
        "audio.microphone" => {
            let _ = select_microphone(state.app.clone(), state, value);
        }
        "soundboard.global_hotkeys" => {
            sync_hotkeys(&state.app, &state).map_err(err_string)?;
        }
        _ => {}
    }
    Ok(())
}
