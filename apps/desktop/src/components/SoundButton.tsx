import { useEffect, useState } from "react";

import { useSoundboardStore } from "../stores/soundboardStore";
import type { SoundClip } from "../lib/types";

interface SoundButtonProps {
  clip: SoundClip;
  onEdit: (clip: SoundClip) => void;
}

export function formatDuration(ms: number): string {
  const totalSec = Math.max(0, Math.ceil(ms / 1000));
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function useRemainingMs(playing: boolean, startedAt: number | undefined, durationMs: number) {
  const [now, setNow] = useState(() => performance.now());

  useEffect(() => {
    if (!playing || startedAt === undefined) return;
    setNow(performance.now());
    const id = window.setInterval(() => setNow(performance.now()), 200);
    return () => window.clearInterval(id);
  }, [playing, startedAt]);

  if (!playing || startedAt === undefined || durationMs <= 0) return null;
  return Math.max(0, durationMs - (now - startedAt));
}

export function SoundButton({ clip, onEdit }: SoundButtonProps) {
  const playing = useSoundboardStore((s) => s.playing[clip.id] === true);
  const startedAt = useSoundboardStore((s) => s.playingStartedAt[clip.id]);
  const playClip = useSoundboardStore((s) => s.playClip);
  const stopClip = useSoundboardStore((s) => s.stopClip);
  const remainingMs = useRemainingMs(playing, startedAt, clip.duration_ms);

  if (!clip.enabled) {
    return (
      <button
        type="button"
        disabled
        className="flex min-h-24 flex-col items-center justify-center gap-1 rounded-lg border border-slate-800 bg-slate-900/50 px-4 py-6 text-slate-600"
        onContextMenu={(event) => {
          event.preventDefault();
          onEdit(clip);
        }}
        title={`${clip.name} (disabled)`}
      >
        <span className="text-base font-semibold">{clip.name}</span>
        {clip.shortcut ? (
          <kbd className="rounded bg-slate-800 px-1.5 py-0.5 text-xs">{clip.shortcut}</kbd>
        ) : null}
      </button>
    );
  }

  const statusLabel =
    remainingMs !== null
      ? formatDuration(remainingMs)
      : playing
        ? "Playing"
        : clip.duration_ms > 0
          ? formatDuration(clip.duration_ms)
          : null;

  return (
    <button
      type="button"
      onClick={() => (playing ? void stopClip(clip.id) : void playClip(clip.id))}
      onContextMenu={(event) => {
        event.preventDefault();
        onEdit(clip);
      }}
      aria-pressed={playing}
      title={`${clip.name}${clip.shortcut ? ` (${clip.shortcut})` : ""} — right-click to edit`}
      className={`group relative flex min-h-24 flex-col items-center justify-center gap-1 overflow-hidden rounded-lg border px-4 py-6 text-base font-semibold transition-colors ${
        playing
          ? "border-accent bg-accent/20 text-white shadow-[inset_0_0_0_2px_var(--color-accent)]"
          : "border-slate-700 bg-slate-800 text-slate-100 hover:border-slate-500 hover:bg-slate-700"
      }`}
    >
      {playing && clip.duration_ms > 0 ? (
        <span
          aria-hidden
          className="pointer-events-none absolute inset-x-0 bottom-0 h-1 bg-slate-950/40"
        >
          <span
            className="block h-full bg-accent transition-[width] duration-200 ease-linear"
            style={{
              width: `${Math.min(100, Math.max(0, ((clip.duration_ms - (remainingMs ?? 0)) / clip.duration_ms) * 100))}%`,
            }}
          />
        </span>
      ) : null}
      <span className="text-center leading-tight">{clip.name}</span>
      <span className="flex items-center gap-2">
        {clip.shortcut ? (
          <kbd className="rounded bg-slate-950/60 px-1.5 py-0.5 text-xs text-slate-300">
            {clip.shortcut}
          </kbd>
        ) : null}
        {statusLabel ? (
          <span
            className={`tabular-nums text-xs tracking-wider ${
              playing ? "font-bold text-accent" : "font-medium text-slate-400"
            }`}
            role={playing ? "status" : undefined}
          >
            {statusLabel}
          </span>
        ) : null}
      </span>
    </button>
  );
}
