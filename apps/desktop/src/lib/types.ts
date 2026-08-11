export interface SoundClip {
  id: number;
  name: string;
  file_path: string;
  category_id: number | null;
  duration_ms: number;
  volume: number;
  shortcut: string | null;
  enabled: boolean;
  sort_order: number;
}

export interface NewSoundClip {
  name: string;
  file_path: string;
  category_id?: number | null;
  volume?: number;
  shortcut?: string | null;
  enabled?: boolean;
  sort_order?: number;
}

export interface UpdateSoundClip {
  id: number;
  name?: string;
  category_id?: number | null;
  volume?: number;
  shortcut?: string | null;
  enabled?: boolean;
  sort_order?: number;
}

export interface Category {
  id: number;
  name: string;
  sort_order: number;
}

export interface NewCategory {
  name: string;
  sort_order?: number;
}

export interface AudioDevice {
  id: number;
  name: string;
  node_name: string;
  description: string;
  media_class: string;
}

export interface MicStatus {
  connected: boolean;
  muted: boolean;
  device_id: number | null;
  device_name: string | null;
  error: string | null;
}

export interface VirtualMicStatus {
  running: boolean;
  node_id: number | null;
  name: string;
  error: string | null;
}

export interface AudioStatus {
  engine_running: boolean;
  microphone: MicStatus;
  virtual_microphone: VirtualMicStatus;
  sample_rate: number;
  channels: number;
  master_volume: number;
  mic_volume: number;
  soundboard_volume: number;
  monitor_enabled: boolean;
}

export interface AudioSettings {
  microphone: string | null;
  microphone_volume: number;
  soundboard_volume: number;
  master_volume: number;
  monitor_enabled: boolean;
}

export interface AppearanceSettings {
  theme: string;
  button_size: string;
}

export interface SoundboardSettings {
  default_category_id: number | null;
  global_hotkeys: boolean;
}

export interface StartupSettings {
  start_audio_engine: boolean;
}

export interface AppSettings {
  audio: AudioSettings;
  appearance: AppearanceSettings;
  soundboard: SoundboardSettings;
  startup: StartupSettings;
}

export interface SoundEventPayload {
  clip_id: number;
  name: string;
  reason?: string;
}

export interface AudioErrorPayload {
  message: string;
}
