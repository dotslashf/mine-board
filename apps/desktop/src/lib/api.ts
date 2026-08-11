import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AppSettings,
  AudioDevice,
  AudioStatus,
  Category,
  MicStatus,
  NewCategory,
  NewSoundClip,
  SoundClip,
  UpdateSoundClip,
  VirtualMicStatus,
} from "./types";

export const EVENTS = {
  audioDeviceChanged: "audio-device-changed",
  micStatusChanged: "microphone-status-changed",
  virtualMicStatusChanged: "virtual-mic-status-changed",
  soundStarted: "sound-started",
  soundStopped: "sound-stopped",
  audioError: "audio-error",
} as const;

export const api = {
  // Audio
  getAudioDevices: () => invoke<AudioDevice[]>("get_audio_devices"),
  getInputDevices: () => invoke<AudioDevice[]>("get_input_devices"),
  selectMicrophone: (deviceId: string) => invoke<void>("select_microphone", { deviceId }),
  getAudioStatus: () => invoke<AudioStatus>("get_audio_status"),
  startAudioEngine: () => invoke<void>("start_audio_engine"),
  stopAudioEngine: () => invoke<void>("stop_audio_engine"),
  getVirtualMicrophoneStatus: () => invoke<VirtualMicStatus>("get_virtual_microphone_status"),
  getMicStatus: () => invoke<MicStatus>("get_microphone_status"),
  setMicVolume: (volume: number) => invoke<void>("set_mic_volume", { volume }),
  setSoundboardVolume: (volume: number) => invoke<void>("set_soundboard_volume", { volume }),
  setMasterVolume: (volume: number) => invoke<void>("set_master_volume", { volume }),
  setMicMuted: (muted: boolean) => invoke<void>("set_mic_muted", { muted }),
  setMonitorEnabled: (enabled: boolean) => invoke<void>("set_monitor_enabled", { enabled }),

  // Soundboard
  playSound: (clipId: number) => invoke<void>("play_sound", { clipId }),
  stopSound: (clipId: number) => invoke<void>("stop_sound", { clipId }),
  stopAllSounds: () => invoke<void>("stop_all_sounds"),

  // Clips
  listSoundClips: () => invoke<SoundClip[]>("list_sound_clips"),
  createSoundClip: (clip: NewSoundClip) => invoke<SoundClip>("create_sound_clip", { clip }),
  updateSoundClip: (clip: UpdateSoundClip) => invoke<SoundClip>("update_sound_clip", { clip }),
  deleteSoundClip: (clipId: number) => invoke<void>("delete_sound_clip", { clipId }),
  importSoundClip: (sourcePath: string, name?: string) =>
    invoke<SoundClip>("import_sound_clip", { sourcePath, name }),
  importAudioFile: (sourcePath: string) =>
    invoke<string>("import_audio_file", { sourcePath }),
  replaceSoundClipAudio: (clipId: number, sourcePath: string) =>
    invoke<SoundClip>("replace_sound_clip_audio", { clipId, sourcePath }),

  // Categories
  listCategories: () => invoke<Category[]>("list_categories"),
  createCategory: (category: NewCategory) => invoke<Category>("create_category", { category }),
  updateCategory: (id: number, name: string) =>
    invoke<Category>("update_category", { id, name }),
  deleteCategory: (id: number) => invoke<void>("delete_category", { id }),

  // Settings
  getAppSettings: () => invoke<AppSettings>("get_app_settings"),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
};

export function onEvent<T>(event: string, handler: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(event, (event) => handler(event.payload));
}
