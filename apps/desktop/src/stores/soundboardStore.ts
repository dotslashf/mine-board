import { create } from "zustand";

import { api, EVENTS, onEvent } from "../lib/api";
import type { Category, NewSoundClip, SoundClip, UpdateSoundClip } from "../lib/types";

interface SoundboardState {
  clips: SoundClip[];
  categories: Category[];
  playing: Record<number, boolean>;
  /** `performance.now()` when playback started, keyed by clip id. */
  playingStartedAt: Record<number, number>;
  selectedCategoryId: number | null;
  loading: boolean;
  error: string | null;

  init: () => Promise<void>;
  loadClips: () => Promise<void>;
  loadCategories: () => Promise<void>;
  selectCategory: (id: number | null) => void;
  playClip: (clipId: number) => Promise<void>;
  stopClip: (clipId: number) => Promise<void>;
  stopAll: () => Promise<void>;
  importClip: (sourcePath: string, name?: string) => Promise<SoundClip>;
  createClip: (clip: NewSoundClip) => Promise<SoundClip>;
  updateClip: (clip: UpdateSoundClip) => Promise<void>;
  deleteClip: (clipId: number) => Promise<void>;
  createCategory: (name: string) => Promise<Category>;
  deleteCategory: (id: number) => Promise<void>;
  setError: (error: string | null) => void;
}

export const useSoundboardStore = create<SoundboardState>((set, get) => ({
  clips: [],
  categories: [],
  playing: {},
  playingStartedAt: {},
  selectedCategoryId: null,
  loading: false,
  error: null,

  init: async () => {
    onEvent<{ clip_id: number; name: string }>(EVENTS.soundStarted, ({ clip_id }) => {
      set({
        playing: { ...get().playing, [clip_id]: true },
        playingStartedAt: { ...get().playingStartedAt, [clip_id]: performance.now() },
      });
    });
    onEvent<{ clip_id: number; name: string; reason?: string }>(EVENTS.soundStopped, ({ clip_id }) => {
      const playing = { ...get().playing };
      const playingStartedAt = { ...get().playingStartedAt };
      delete playing[clip_id];
      delete playingStartedAt[clip_id];
      set({ playing, playingStartedAt });
    });
    onEvent<{ message: string }>(EVENTS.audioError, ({ message }) =>
      set({ error: message }),
    );

    await Promise.all([get().loadCategories(), get().loadClips()]);
  },

  loadClips: async () => {
    set({ loading: true });
    try {
      const clips = await api.listSoundClips();
      set({ clips });
    } catch (error) {
      set({ error: String(error) });
    } finally {
      set({ loading: false });
    }
  },

  loadCategories: async () => {
    const categories = await api.listCategories().catch(() => []);
    set({ categories });
  },

  selectCategory: (id) => set({ selectedCategoryId: id }),

  playClip: async (clipId) => {
    await api.playSound(clipId).catch((error) => set({ error: String(error) }));
  },

  stopClip: async (clipId) => {
    await api.stopSound(clipId).catch((error) => set({ error: String(error) }));
  },

  stopAll: async () => {
    await api.stopAllSounds().catch((error) => set({ error: String(error) }));
    set({ playing: {}, playingStartedAt: {} });
  },

  importClip: async (sourcePath, name) => {
    const clip = await api.importSoundClip(sourcePath, name);
    await get().loadClips();
    return clip;
  },

  createClip: async (clip) => {
    const created = await api.createSoundClip(clip);
    await get().loadClips();
    return created;
  },

  updateClip: async (clip) => {
    await api.updateSoundClip(clip);
    await get().loadClips();
  },

  deleteClip: async (clipId) => {
    await api.deleteSoundClip(clipId);
    const playing = { ...get().playing };
    const playingStartedAt = { ...get().playingStartedAt };
    delete playing[clipId];
    delete playingStartedAt[clipId];
    set({ playing, playingStartedAt });
    await get().loadClips();
  },

  createCategory: async (name) => {
    const category = await api.createCategory({ name });
    await get().loadCategories();
    return category;
  },

  deleteCategory: async (id) => {
    await api.deleteCategory(id);
    await get().loadCategories();
    await get().loadClips();
  },

  setError: (error) => set({ error }),
}));
