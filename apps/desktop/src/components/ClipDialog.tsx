import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";

import { api } from "../lib/api";
import type { Category, SoundClip } from "../lib/types";
import { useSoundboardStore } from "../stores/soundboardStore";

export type DialogState =
  | { mode: "create"; sourcePath: string; suggestedName: string }
  | { mode: "edit"; clip: SoundClip }
  | null;

interface ClipDialogProps {
  state: DialogState;
  onClose: () => void;
}

const AUDIO_FILTERS = [{ name: "WAV Audio", extensions: ["wav"] }];

export async function pickAudioFile(): Promise<{ sourcePath: string; name: string } | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: AUDIO_FILTERS,
  });
  if (typeof selected !== "string") return null;
  const name = selected.split(/[\\/]/).pop()?.replace(/\.[^.]+$/, "") || "New clip";
  return { sourcePath: selected, name };
}

export function ClipDialog({ state, onClose }: ClipDialogProps) {
  const categories = useSoundboardStore((s) => s.categories);
  const createClip = useSoundboardStore((s) => s.createClip);
  const updateClip = useSoundboardStore((s) => s.updateClip);
  const deleteClip = useSoundboardStore((s) => s.deleteClip);
  const createCategory = useSoundboardStore((s) => s.createCategory);

  const [name, setName] = useState("");
  const [volume, setVolume] = useState(1.0);
  const [shortcut, setShortcut] = useState("");
  const [categoryId, setCategoryId] = useState<number | "none">("none");
  const [enabled, setEnabled] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (state?.mode === "edit") {
      setName(state.clip.name);
      setVolume(state.clip.volume);
      setShortcut(state.clip.shortcut ?? "");
      setCategoryId(state.clip.category_id ?? "none");
      setEnabled(state.clip.enabled);
    } else if (state?.mode === "create") {
      setName(state.suggestedName);
    }
  }, [state]);

  if (!state) return null;

  const isEdit = state.mode === "edit";

  const handleSave = async () => {
    const trimmed = name.trim();
    if (!trimmed) {
      setError("Name is required");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      if (isEdit) {
        await updateClip({
          id: state.clip.id,
          name: trimmed,
          volume,
          shortcut: shortcut.trim() || null,
          category_id: categoryId === "none" ? null : categoryId,
          enabled,
        });
      } else {
        const filePath = await api.importAudioFile(state.sourcePath);
        await createClip({
          name: trimmed,
          file_path: filePath,
          volume,
          shortcut: shortcut.trim() || null,
          category_id: categoryId === "none" ? null : categoryId,
          enabled,
        });
      }
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleReplaceAudio = async () => {
    if (!isEdit) return;
    const picked = await pickAudioFile();
    if (!picked) return;
    setBusy(true);
    setError(null);
    try {
      await api.replaceSoundClipAudio(state.clip.id, picked.sourcePath);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const handleDelete = async () => {
    if (!isEdit) return;
    const confirmed = window.confirm(`Delete clip "${state.clip.name}"?`);
    if (!confirmed) return;
    setBusy(true);
    setError(null);
    try {
      await deleteClip(state.clip.id);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4"
      role="dialog"
      aria-modal="true"
      aria-label={isEdit ? `Edit ${state.clip.name}` : "Add clip"}
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <form
        className="w-full max-w-md rounded-xl border border-slate-700 bg-slate-900 p-5"
        onSubmit={(event) => {
          event.preventDefault();
          void handleSave();
        }}
      >
        <h2 className="mb-4 text-lg font-semibold">{isEdit ? "Edit Clip" : "Add Clip"}</h2>

        <div className="mb-3">
          <label className="mb-1 block text-sm text-slate-300" htmlFor="clip-name">
            Name
          </label>
          <input
            id="clip-name"
            value={name}
            onChange={(event) => setName(event.target.value)}
            className="w-full rounded-md border border-slate-600 bg-slate-950 px-2 py-1.5"
          />
        </div>

        <div className="mb-3">
          <label className="mb-1 block text-sm text-slate-300" htmlFor="clip-volume">
            Volume: {Math.round(volume * 100)}%
          </label>
          <input
            id="clip-volume"
            type="range"
            min={0}
            max={2}
            step={0.05}
            value={volume}
            onChange={(event) => setVolume(Number(event.target.value))}
            className="w-full"
          />
        </div>

        <div className="mb-3">
          <label className="mb-1 block text-sm text-slate-300" htmlFor="clip-shortcut">
            Global shortcut
          </label>
          <input
            id="clip-shortcut"
            value={shortcut}
            onChange={(event) => setShortcut(event.target.value)}
            placeholder="e.g. F1 or Ctrl+Shift+K"
            className="w-full rounded-md border border-slate-600 bg-slate-950 px-2 py-1.5"
          />
        </div>

        <div className="mb-3">
          <label className="mb-1 block text-sm text-slate-300" htmlFor="clip-category">
            Category
          </label>
          <select
            id="clip-category"
            value={categoryId}
            onChange={(event) => {
              const value = event.target.value;
              if (value === "__new__") {
                const name = window.prompt("New category name");
                if (name) void createCategory(name);
                return;
              }
              setCategoryId(value === "none" ? "none" : Number(value));
            }}
            className="w-full rounded-md border border-slate-600 bg-slate-950 px-2 py-1.5"
          >
            <option value="none">No category</option>
            {categories.map((category: Category) => (
              <option key={category.id} value={category.id}>
                {category.name}
              </option>
            ))}
            <option value="__new__">+ New category…</option>
          </select>
        </div>

        <label className="mb-4 flex items-center gap-2 text-sm text-slate-300">
          <input
            type="checkbox"
            checked={enabled}
            onChange={(event) => setEnabled(event.target.checked)}
            className="h-4 w-4"
          />
          Enabled
        </label>

        {error ? (
          <p className="mb-3 text-sm text-rose-400" role="alert">
            {error}
          </p>
        ) : null}

        <div className="flex items-center justify-between gap-2">
          <div className="flex gap-2">
            {isEdit ? (
              <>
                <button
                  type="button"
                  onClick={() => void handleReplaceAudio()}
                  disabled={busy}
                  className="rounded-md bg-slate-700 px-3 py-1.5 text-sm text-white hover:bg-slate-600"
                >
                  Replace audio
                </button>
                <button
                  type="button"
                  onClick={() => void handleDelete()}
                  disabled={busy}
                  className="rounded-md bg-rose-900/60 px-3 py-1.5 text-sm text-rose-200 hover:bg-rose-800"
                >
                  Delete
                </button>
              </>
            ) : null}
          </div>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={onClose}
              className="rounded-md px-3 py-1.5 text-sm text-slate-300 hover:text-white"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={busy}
              className="rounded-md bg-accent px-3 py-1.5 text-sm font-semibold text-slate-950 hover:opacity-90"
            >
              {isEdit ? "Save" : "Add"}
            </button>
          </div>
        </div>
      </form>
    </div>
  );
}
