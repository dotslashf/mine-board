use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundClip {
    pub id: i64,
    pub name: String,
    pub category_id: Option<i64>,
    pub file_path: String,
    pub duration_ms: i64,
    pub volume: f64,
    pub shortcut: Option<String>,
    pub sort_order: i64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NewSoundClip {
    pub name: String,
    pub category_id: Option<i64>,
    pub file_path: String,
    pub duration_ms: i64,
    pub volume: f64,
    pub shortcut: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UpdateSoundClip {
    pub id: i64,
    pub name: Option<String>,
    pub category_id: Option<Option<i64>>,
    pub file_path: Option<String>,
    pub duration_ms: Option<i64>,
    pub volume: Option<f64>,
    pub shortcut: Option<Option<String>>,
    pub sort_order: Option<i64>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
    pub is_default: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NewCategory {
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub node_name: String,
    pub description: String,
    pub media_class: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MicStatus {
    pub connected: bool,
    pub muted: bool,
    pub device_id: Option<u32>,
    pub device_name: Option<String>,
    pub error: Option<String>,
}

impl Default for MicStatus {
    fn default() -> Self {
        Self {
            connected: false,
            muted: false,
            device_id: None,
            device_name: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VirtualMicStatus {
    pub running: bool,
    pub node_id: Option<u32>,
    pub name: String,
    pub error: Option<String>,
}

impl Default for VirtualMicStatus {
    fn default() -> Self {
        Self {
            running: false,
            node_id: None,
            name: "ATCS Soundboard Virtual Mic".into(),
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStatus {
    pub engine_running: bool,
    pub microphone: MicStatus,
    pub virtual_microphone: VirtualMicStatus,
    pub sample_rate: u32,
    pub channels: u16,
    pub master_volume: f64,
    pub mic_volume: f64,
    pub soundboard_volume: f64,
    pub monitor_enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub audio: AudioSettings,
    pub appearance: AppearanceSettings,
    pub soundboard: SoundboardSettings,
    pub startup: StartupSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioSettings {
    /// PipeWire node id or node name, stored as a plain string ("47"). The
    /// settings store parses it as JSON, so tolerate numeric values too.
    #[serde(deserialize_with = "deserialize_option_string_or_number")]
    pub microphone: Option<String>,
    pub microphone_volume: f64,
    pub soundboard_volume: f64,
    pub master_volume: f64,
    pub monitor_enabled: bool,
}

fn deserialize_option_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.map(|v| match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }))
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            microphone: None,
            microphone_volume: 1.0,
            soundboard_volume: 1.0,
            master_volume: 0.9,
            monitor_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceSettings {
    pub theme: String,
    pub button_size: String,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            button_size: "large".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundboardSettings {
    pub global_hotkeys: bool,
    pub default_category_id: Option<i64>,
}

impl Default for SoundboardSettings {
    fn default() -> Self {
        Self {
            global_hotkeys: true,
            default_category_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StartupSettings {
    pub start_audio_engine: bool,
    pub launch_at_login: bool,
}

impl Default for StartupSettings {
    fn default() -> Self {
        Self {
            start_audio_engine: true,
            launch_at_login: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundEventPayload {
    pub clip_id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundStoppedPayload {
    pub clip_id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioErrorPayload {
    pub message: String,
}
