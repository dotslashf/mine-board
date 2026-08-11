import { mock } from "bun:test";

// Shared mock of src/lib/api. `mock.module` is hoisted per test file, so each
// test file must call it directly (it cannot be registered from a helper):
//
//   mock.module("../src/lib/api", apiFactory);
//
// Then assert on `apiMock`.

export const apiMock = {
  // Audio
  getAudioDevices: mock(() => Promise.resolve([])),
  getInputDevices: mock(() => Promise.resolve([])),
  selectMicrophone: mock(() => Promise.resolve()),
  getAudioStatus: mock(() => Promise.resolve(null)),
  startAudioEngine: mock(() => Promise.resolve()),
  stopAudioEngine: mock(() => Promise.resolve()),
  getVirtualMicrophoneStatus: mock(() => Promise.resolve({})),
  getMicStatus: mock(() => Promise.resolve({})),
  setMicVolume: mock(() => Promise.resolve()),
  setSoundboardVolume: mock(() => Promise.resolve()),
  setMasterVolume: mock(() => Promise.resolve()),
  setMicMuted: mock(() => Promise.resolve()),
  setMonitorEnabled: mock(() => Promise.resolve()),

  // Playback
  playSound: mock(() => Promise.resolve()),
  stopSound: mock(() => Promise.resolve()),
  stopAllSounds: mock(() => Promise.resolve()),

  // Clips
  listSoundClips: mock(() => Promise.resolve([])),
  createSoundClip: mock((clip: Record<string, unknown>) =>
    Promise.resolve({ id: 1, ...clip }),
  ),
  updateSoundClip: mock(() => Promise.resolve({})),
  deleteSoundClip: mock(() => Promise.resolve()),
  importSoundClip: mock(() => Promise.resolve({ id: 1 })),
  importAudioFile: mock(() => Promise.resolve("/tmp/mocked-clip.wav")),
  replaceSoundClipAudio: mock(() => Promise.resolve({ id: 1 })),

  // Categories
  listCategories: mock(() => Promise.resolve([])),
  createCategory: mock(() => Promise.resolve({ id: 1, name: "New", sort_order: 0 })),
  updateCategory: mock(() => Promise.resolve({ id: 1, name: "New", sort_order: 0 })),
  deleteCategory: mock(() => Promise.resolve()),

  // Settings
  getAppSettings: mock(() => Promise.resolve({})),
  setSetting: mock(() => Promise.resolve()),
};

export function apiFactory() {
  return {
    api: apiMock,
    EVENTS: {
      audioDeviceChanged: "audio-device-changed",
      micStatusChanged: "microphone-status-changed",
      virtualMicStatusChanged: "virtual-mic-status-changed",
      soundStarted: "sound-started",
      soundStopped: "sound-stopped",
      audioError: "audio-error",
    },
    onEvent: mock(() => Promise.resolve(() => {})),
  };
}
