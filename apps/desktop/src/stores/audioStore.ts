import { create } from "zustand";

import { api, EVENTS, onEvent } from "../lib/api";
import type { AudioDevice, AudioStatus, MicStatus, VirtualMicStatus } from "../lib/types";

interface AudioState {
  devices: AudioDevice[];
  selectedDeviceId: string | null;
  status: AudioStatus | null;
  micStatus: MicStatus | null;
  virtualMicStatus: VirtualMicStatus | null;
  engineRunning: boolean;
  error: string | null;
  initialized: boolean;

  init: () => Promise<void>;
  refreshDevices: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  selectMicrophone: (deviceId: string) => Promise<void>;
  setMicVolume: (volume: number) => Promise<void>;
  setSoundboardVolume: (volume: number) => Promise<void>;
  setMasterVolume: (volume: number) => Promise<void>;
  setMicMuted: (muted: boolean) => Promise<void>;
  setMonitorEnabled: (enabled: boolean) => Promise<void>;
  startEngine: () => Promise<void>;
  stopEngine: () => Promise<void>;
  clearError: () => void;
}

export const useAudioStore = create<AudioState>((set, get) => ({
  devices: [],
  selectedDeviceId: null,
  status: null,
  micStatus: null,
  virtualMicStatus: null,
  engineRunning: false,
  error: null,
  initialized: false,

  init: async () => {
    if (get().initialized) return;

    const unlisteners = [
      onEvent<AudioDevice[]>(EVENTS.audioDeviceChanged, (devices) =>
        set({ devices }),
      ),
      onEvent<MicStatus>(EVENTS.micStatusChanged, (micStatus) =>
        set({ micStatus }),
      ),
      onEvent<VirtualMicStatus>(EVENTS.virtualMicStatusChanged, (virtualMicStatus) =>
        set({ virtualMicStatus }),
      ),
      onEvent<{ message: string }>(EVENTS.audioError, ({ message }) =>
        set({ error: message }),
      ),
    ];

    const [devices, status] = await Promise.all([
      api.getAudioDevices().catch(() => []),
      api.getAudioStatus().catch(() => null),
    ]);

    const selectedDeviceId =
      status?.microphone?.device_id != null ? String(status.microphone.device_id) : null;

    set({
      devices,
      status,
      micStatus: status?.microphone ?? null,
      virtualMicStatus: status?.virtual_microphone ?? null,
      engineRunning: status?.engine_running ?? false,
      selectedDeviceId,
      initialized: true,
    });

    get().refreshStatus();
  },

  refreshDevices: async () => {
    const devices = await api.getAudioDevices();
    set({ devices });
  },

  refreshStatus: async () => {
    const status = await api.getAudioStatus().catch(() => null);
    if (!status) return;
    set({
      status,
      micStatus: status.microphone,
      virtualMicStatus: status.virtual_microphone,
      engineRunning: status.engine_running,
      selectedDeviceId:
        status.microphone.device_id != null
          ? String(status.microphone.device_id)
          : get().selectedDeviceId,
    });
  },

  selectMicrophone: async (deviceId) => {
    await api.selectMicrophone(deviceId);
    set({ selectedDeviceId: deviceId });
    await get().refreshStatus();
  },

  setMicVolume: async (volume) => {
    await api.setMicVolume(volume);
    const status = get().status;
    if (status) set({ status: { ...status, mic_volume: volume } });
  },

  setSoundboardVolume: async (volume) => {
    await api.setSoundboardVolume(volume);
    const status = get().status;
    if (status) set({ status: { ...status, soundboard_volume: volume } });
  },

  setMasterVolume: async (volume) => {
    await api.setMasterVolume(volume);
    const status = get().status;
    if (status) set({ status: { ...status, master_volume: volume } });
  },

  setMicMuted: async (muted) => {
    await api.setMicMuted(muted);
    const status = get().status;
    if (status) {
      set({
        status: {
          ...status,
          microphone: { ...status.microphone, muted },
        },
      });
    }
  },

  setMonitorEnabled: async (enabled) => {
    await api.setMonitorEnabled(enabled);
    const status = get().status;
    if (status) set({ status: { ...status, monitor_enabled: enabled } });
  },

  startEngine: async () => {
    try {
      await api.startAudioEngine();
      set({ engineRunning: true, error: null });
    } catch (error) {
      set({ error: String(error) });
      throw error;
    }
    await get().refreshStatus();
  },

  stopEngine: async () => {
    try {
      await api.stopAudioEngine();
      set({ engineRunning: false });
    } catch (error) {
      set({ error: String(error) });
      throw error;
    }
    await get().refreshStatus();
  },

  clearError: () => set({ error: null }),
}));
