mod commands;
mod engine;
mod hotkeys;
mod state;

use atcs_soundboard_core::db::Database;
use atcs_soundboard_core::models::NewCategory;
use atcs_soundboard_core::settings;
use tauri::Manager;
use tauri_plugin_global_shortcut::{Shortcut, ShortcutEvent, ShortcutState};

use state::AppState;

fn on_global_shortcut(app: &tauri::AppHandle, shortcut: &Shortcut, event: ShortcutEvent) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let normalized = hotkeys::normalize_shortcut(&shortcut.to_string());
    let clip_id = {
        let map = state.hotkey_map.lock().unwrap();
        map.get(&normalized).copied()
    };
    if let Some(clip_id) = clip_id {
        let _ = commands::play_clip_internal(app, &state, clip_id);
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(on_global_shortcut)
                .build(),
        )
        .setup(|app| {
            let repo = Database::open()?.init()?;

            if repo.list_categories()?.is_empty() {
                repo.create_category(NewCategory {
                    name: "Default".into(),
                    is_default: true,
                })?;
            }

            let settings = settings::load_settings(&repo)?;
            let app_state = AppState {
                db: std::sync::Mutex::new(repo),
                engine: std::sync::Mutex::new(None),
                clip_cache: std::sync::Mutex::new(Default::default()),
                hotkey_map: std::sync::Mutex::new(Default::default()),
                app: app.handle().clone(),
            };
            app.manage(app_state);

            if settings.startup.start_audio_engine {
                let state = app.state::<AppState>();
                let _ = commands::start_audio_engine(app.handle().clone(), state);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Audio
            commands::start_audio_engine,
            commands::stop_audio_engine,
            commands::get_audio_status,
            commands::get_virtual_microphone_status,
            commands::get_microphone_status,
            commands::get_audio_devices,
            commands::get_input_devices,
            commands::select_microphone,
            commands::set_mic_volume,
            commands::set_soundboard_volume,
            commands::set_master_volume,
            commands::set_mic_muted,
            commands::set_monitor_enabled,
            // Playback
            commands::play_sound,
            commands::stop_sound,
            commands::stop_all_sounds,
            // Clips
            commands::list_sound_clips,
            commands::create_sound_clip,
            commands::update_sound_clip,
            commands::delete_sound_clip,
            commands::import_audio_file,
            commands::import_sound_clip,
            commands::replace_sound_clip_audio,
            // Categories
            commands::list_categories,
            commands::create_category,
            commands::update_category,
            commands::delete_category,
            // Settings
            commands::get_app_settings,
            commands::set_setting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ATCS Soundboard");
}
