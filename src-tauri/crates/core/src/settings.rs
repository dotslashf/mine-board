use serde_json::{Map, Value};

use crate::db::Repository;
use crate::errors::Result;
use crate::models::{AppSettings, AudioSettings};

/// Load settings persisted as flat `key.path` rows into an [`AppSettings`].
pub fn load_settings(repo: &Repository) -> Result<AppSettings> {
    let all = repo.all_settings()?;
    let mut map = Map::new();
    for (key, value) in all {
        // Values are stored as JSON when possible ("1.0", "true"), but plain
        // strings like "dark" or "47" are stored raw. Accept both: parse as
        // JSON, falling back to the raw string so nothing is ever dropped.
        let v = serde_json::from_str::<Value>(&value).unwrap_or_else(|_| Value::String(value));
        unflatten_into(&mut map, &key, v);
    }
    Ok(serde_json::from_value(Value::Object(map))?)
}

/// Persist every non-`None` leaf of `settings` as a flat `key.path` row.
pub fn save_settings(repo: &Repository, settings: &AppSettings) -> Result<()> {
    let value = serde_json::to_value(settings)?;
    let mut flat: Vec<(String, Value)> = Vec::new();
    flatten(&mut flat, "", &value);
    for (key, value) in flat {
        if value.is_null() {
            continue;
        }
        repo.set_setting(&key, &value.to_string())?;
    }
    Ok(())
}

/// Load only the audio-related keys (used by the engine without touching the
/// rest of the settings map).
pub fn load_audio_settings(repo: &Repository) -> Result<AudioSettings> {
    Ok(load_settings(repo)?.audio)
}

fn flatten(out: &mut Vec<(String, Value)>, prefix: &str, value: &Value) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(out, &path, child);
            }
        }
        other => out.push((prefix.to_string(), other.clone())),
    }
}

fn unflatten_into(map: &mut Map<String, Value>, path: &str, value: Value) {
    let mut parts = path.split('.');
    let head = parts.next().expect("non-empty key");
    let rest = parts.collect::<Vec<_>>().join(".");
    if rest.is_empty() {
        // Empty string encodes "no value" (e.g. no selected microphone).
        if value.as_str() != Some("") {
            map.insert(head.to_string(), value);
        }
        return;
    }
    let child = map
        .entry(head.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if let Value::Object(child_map) = child {
        unflatten_into(child_map, &rest, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Repository;

    fn repo() -> Repository {
        Repository::in_memory().unwrap()
    }

    #[test]
    fn defaults_when_empty() {
        let repo = repo();
        let settings = load_settings(&repo).unwrap();
        assert_eq!(settings.audio.master_volume, 0.9);
        assert_eq!(settings.audio.microphone_volume, 1.0);
        assert_eq!(settings.appearance.button_size, "large");
        assert!(settings.startup.start_audio_engine);
        assert!(settings.soundboard.global_hotkeys);
    }

    #[test]
    fn roundtrip() {
        let repo = repo();
        let mut settings = load_settings(&repo).unwrap();
        settings.audio.microphone_volume = 0.5;
        settings.audio.microphone = Some("alsa_input.pci-0000".into());
        settings.soundboard.default_category_id = Some(7);
        settings.appearance.button_size = "compact".into();
        settings.startup.launch_at_login = true;
        save_settings(&repo, &settings).unwrap();

        let loaded = load_settings(&repo).unwrap();
        assert_eq!(loaded.audio.microphone_volume, 0.5);
        assert_eq!(
            loaded.audio.microphone.as_deref(),
            Some("alsa_input.pci-0000")
        );
        assert_eq!(loaded.soundboard.default_category_id, Some(7));
        assert_eq!(loaded.appearance.button_size, "compact");
        assert!(loaded.startup.launch_at_login);

        // persisted flat rows exist
        assert!(repo.get_setting("audio.microphone").unwrap().is_some());
    }

    #[test]
    fn save_respects_nulls() {
        let repo = repo();
        let settings = load_settings(&repo).unwrap();
        save_settings(&repo, &settings).unwrap();
        assert!(repo.get_setting("audio.microphone_device_id").unwrap().is_none());
        assert!(repo.get_setting("soundboard.default_category_id").unwrap().is_none());
    }

    #[test]
    fn numeric_microphone_value_loads() {
        // Device ids are persisted as plain numeric strings ("47"); the
        // settings loader must not fail on them (regression: setup panic
        // "invalid type: integer 47, expected a string").
        let repo = repo();
        repo.set_setting("audio.microphone", "47").unwrap();
        let settings = load_settings(&repo).unwrap();
        assert_eq!(settings.audio.microphone.as_deref(), Some("47"));
    }

    #[test]
    fn plain_string_values_load() {
        // Non-JSON strings ("dark") must round-trip instead of being dropped.
        let repo = repo();
        repo.set_setting("appearance.theme", "dark").unwrap();
        let settings = load_settings(&repo).unwrap();
        assert_eq!(settings.appearance.theme, "dark");
    }
}
