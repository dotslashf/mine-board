import { useAudioStore } from "../stores/audioStore";

function StatusDot({ ok }: { ok: boolean }) {
  return (
    <span
      aria-hidden
      className={`inline-block h-2.5 w-2.5 rounded-full ${
        ok ? "bg-emerald-400" : "bg-rose-500"
      }`}
    />
  );
}

export function StatusBar() {
  const micStatus = useAudioStore((s) => s.micStatus);
  const virtualMicStatus = useAudioStore((s) => s.virtualMicStatus);
  const engineRunning = useAudioStore((s) => s.engineRunning);
  const error = useAudioStore((s) => s.error);
  const selectedDeviceId = useAudioStore((s) => s.selectedDeviceId);

  const micName = micStatus?.device_name ?? null;

  return (
    <footer className="flex items-center justify-between border-t border-slate-800 bg-slate-900 px-4 py-2 text-xs text-slate-400">
      <div className="flex items-center gap-5">
        <span className="flex items-center gap-2" title={micName ?? "No microphone selected"}>
          <StatusDot ok={Boolean(micStatus?.connected)} />
          MIC: {micStatus?.connected ? micName ?? "Connected" : "Disconnected"}
          {micStatus?.muted ? " (muted)" : ""}
        </span>
        <span
          className="flex items-center gap-2"
          title={virtualMicStatus?.error ?? "ATCS Soundboard Virtual Mic"}
        >
          <StatusDot ok={Boolean(virtualMicStatus?.running)} />
          VIRTUAL MIC: {virtualMicStatus?.running ? "Running" : "Stopped"}
        </span>
        <span className="flex items-center gap-2">
          <StatusDot ok={engineRunning} />
          ENGINE: {engineRunning ? "Running" : "Stopped"}
        </span>
        {selectedDeviceId != null && (
          <span className="text-slate-500">Device {selectedDeviceId}</span>
        )}
      </div>
      {error ? (
        <span className="max-w-1/2 truncate text-rose-400" role="alert" title={error}>
          {error}
        </span>
      ) : null}
    </footer>
  );
}
