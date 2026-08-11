import { useState } from "react";

import { CategoryBar } from "../components/CategoryBar";
import { ClipDialog, pickAudioFile, type DialogState } from "../components/ClipDialog";
import { SoundButton } from "../components/SoundButton";
import { useSoundboardStore } from "../stores/soundboardStore";

export function SoundboardPage() {
  const clips = useSoundboardStore((s) => s.clips);
  const selectedCategoryId = useSoundboardStore((s) => s.selectedCategoryId);
  const error = useSoundboardStore((s) => s.error);
  const clearError = useSoundboardStore((s) => s.setError);
  const [dialog, setDialog] = useState<DialogState>(null);

  const visibleClips = clips
    .filter((clip) => selectedCategoryId == null || clip.category_id === selectedCategoryId)
    .sort((a, b) => a.sort_order - b.sort_order);

  const handleAdd = async () => {
    const picked = await pickAudioFile();
    if (!picked) return;
    setDialog({ mode: "create", sourcePath: picked.sourcePath, suggestedName: picked.name });
  };

  return (
    <div className="flex h-full flex-col gap-4 p-4">
      {error ? (
        <div
          className="flex items-center justify-between rounded-lg border border-rose-800 bg-rose-950/50 px-3 py-2 text-sm text-rose-200"
          role="alert"
        >
          <span>{error}</span>
          <button type="button" onClick={() => clearError(null)} className="ml-3 text-rose-300">
            Dismiss
          </button>
        </div>
      ) : null}

      <div className="flex items-center justify-between gap-3">
        <CategoryBar />
        <button
          type="button"
          onClick={() => void handleAdd()}
          className="rounded-md bg-accent px-4 py-2 text-sm font-semibold text-slate-950 hover:opacity-90"
        >
          Add clip
        </button>
      </div>

      <div className="grid grid-cols-[repeat(auto-fill,minmax(10rem,1fr))] gap-3 overflow-y-auto pb-2">
        {visibleClips.map((clip) => (
          <SoundButton
            key={clip.id}
            clip={clip}
            onEdit={(clip) => setDialog({ mode: "edit", clip })}
          />
        ))}
        {visibleClips.length === 0 ? (
          <p className="col-span-full py-12 text-center text-slate-500">
            No clips. Use "Add clip" to import your first sound.
          </p>
        ) : null}
      </div>

      <ClipDialog state={dialog} onClose={() => setDialog(null)} />
    </div>
  );
}
