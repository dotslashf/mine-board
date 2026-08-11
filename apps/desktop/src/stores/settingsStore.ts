import { create } from "zustand";

import { api } from "../lib/api";
import type { AppSettings } from "../lib/types";

export const DEFAULT_SETTINGS: AppSettings = {
  audio: {
    microphone: null,
    microphone_volume: 1.0,
    soundboard_volume: 1.0,
    master_volume: 0.9,
    monitor_enabled: false,
  },
  appearance: {
    theme: "dark",
    button_size: "large",
  },
  soundboard: {
    default_category_id: null,
    global_hotkeys: true,
  },
  startup: {
    start_audio_engine: true,
  },
};

interface SettingsState {
  settings: AppSettings;
  loaded: boolean;
  error: string | null;

  load: () => Promise<void>;
  set: (settings: AppSettings) => Promise<void>;
  patch: (patch: Partial<AppSettings>) => Promise<void>;
  setError: (error: string | null) => void;
}

// Map typed settings to the flat key/value store persisted by Rust.
function toKeyValue(settings: AppSettings): Array<[string, string]> {
  const s = settings;
  return [
    ["audio.microphone", s.audio.microphone ?? ""],
    ["audio.microphone_volume", String(s.audio.microphone_volume)],
    ["audio.soundboard_volume", String(s.audio.soundboard_volume)],
    ["audio.master_volume", String(s.audio.master_volume)],
    ["audio.monitor_enabled", String(s.audio.monitor_enabled)],
    ["appearance.theme", s.appearance.theme],
    ["appearance.button_size", s.appearance.button_size],
    ["soundboard.default_category_id", s.soundboard.default_category_id != null ? String(s.soundboard.default_category_id) : ""],
    ["soundboard.global_hotkeys", String(s.soundboard.global_hotkeys)],
    ["startup.start_audio_engine", String(s.startup.start_audio_engine)],
  ];
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: DEFAULT_SETTINGS,
  loaded: false,
  error: null,

  load: async () => {
    try {
      const settings = await api.getAppSettings();
      set({ settings, loaded: true });
    } catch (error) {
      set({ error: String(error), loaded: true });
    }
  },

  set: async (settings) => {
    const kv = toKeyValue(settings);
    for (const [key, value] of kv) {
      await api.setSetting(key, value);
    }
    set({ settings });
  },

  patch: async (patch) => {
    const next = { ...get().settings, ...patch };
    await get().set(next);
  },

  setError: (error) => set({ error }),
}));
