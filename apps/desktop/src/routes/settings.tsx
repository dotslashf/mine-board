import { useAudioStore } from "../stores/audioStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useSoundboardStore } from "../stores/soundboardStore";

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-xl border border-slate-800 bg-slate-900 p-4">
      <h2 className="mb-3 text-base font-semibold text-slate-200">{title}</h2>
      <div className="space-y-3">{children}</div>
    </section>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="flex items-center justify-between gap-4 text-sm text-slate-300">
      <span>{label}</span>
      {children}
    </label>
  );
}

export function SettingsPage() {
  const devices = useAudioStore((s) => s.devices);
  const micStatus = useAudioStore((s) => s.micStatus);
  const virtualMicStatus = useAudioStore((s) => s.virtualMicStatus);
  const status = useAudioStore((s) => s.status);
  const engineRunning = useAudioStore((s) => s.engineRunning);
  const selectMicrophone = useAudioStore((s) => s.selectMicrophone);
  const setMicVolume = useAudioStore((s) => s.setMicVolume);
  const setSoundboardVolume = useAudioStore((s) => s.setSoundboardVolume);
  const setMasterVolume = useAudioStore((s) => s.setMasterVolume);
  const setMicMuted = useAudioStore((s) => s.setMicMuted);
  const setMonitorEnabled = useAudioStore((s) => s.setMonitorEnabled);
  const startEngine = useAudioStore((s) => s.startEngine);
  const stopEngine = useAudioStore((s) => s.stopEngine);

  const categories = useSoundboardStore((s) => s.categories);
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.set);

  return (
    <div className="h-full overflow-y-auto p-4">
      <div className="mx-auto max-w-2xl space-y-4">
        <Section title="Audio Engine">
          <div className="flex items-center justify-between">
            <span className="text-sm text-slate-300">Engine status</span>
            {engineRunning ? (
              <button
                type="button"
                onClick={() => void stopEngine()}
                className="rounded-md bg-rose-800 px-3 py-1.5 text-sm text-white hover:bg-rose-700"
              >
                Stop engine
              </button>
            ) : (
              <button
                type="button"
                onClick={() => void startEngine()}
                className="rounded-md bg-accent px-3 py-1.5 text-sm font-semibold text-slate-950 hover:opacity-90"
              >
                Start engine
              </button>
            )}
          </div>

          <Row label="Virtual microphone">
            <span className={`text-sm ${virtualMicStatus?.running ? "text-emerald-400" : "text-rose-400"}`}>
              {virtualMicStatus?.running ? "Running" : "Stopped"} — {virtualMicStatus?.name}
            </span>
          </Row>

          <Row label="Microphone device">
            <select
              value={micStatus?.device_id != null ? String(micStatus.device_id) : ""}
              onChange={(event) => {
                const value = event.target.value;
                if (value) void selectMicrophone(value);
              }}
              className="rounded-md border border-slate-600 bg-slate-950 px-2 py-1.5"
            >
              <option value="" disabled>
                Select microphone…
              </option>
              {devices.map((device) => (
                <option key={device.id} value={device.id}>
                  {device.description || device.name}
                </option>
              ))}
            </select>
          </Row>

          <Row label="Microphone volume">
            <input
              type="range"
              min={0}
              max={1.5}
              step={0.05}
              value={status?.mic_volume ?? 1}
              onChange={(event) => void setMicVolume(Number(event.target.value))}
              className="w-48"
            />
          </Row>

          <Row label="Mute microphone">
            <input
              type="checkbox"
              checked={micStatus?.muted ?? false}
              onChange={(event) => void setMicMuted(event.target.checked)}
              className="h-4 w-4"
            />
          </Row>

          <Row label="Soundboard volume">
            <input
              type="range"
              min={0}
              max={1.5}
              step={0.05}
              value={status?.soundboard_volume ?? 1}
              onChange={(event) => void setSoundboardVolume(Number(event.target.value))}
              className="w-48"
            />
          </Row>

          <Row label="Master volume">
            <input
              type="range"
              min={0}
              max={1.5}
              step={0.05}
              value={status?.master_volume ?? 0.9}
              onChange={(event) => void setMasterVolume(Number(event.target.value))}
              className="w-48"
            />
          </Row>

          <Row label="Monitor (hear mixed output)">
            <input
              type="checkbox"
              checked={status?.monitor_enabled ?? false}
              onChange={(event) => void setMonitorEnabled(event.target.checked)}
              className="h-4 w-4"
            />
          </Row>
        </Section>

        <Section title="Soundboard">
          <Row label="Global hotkeys">
            <input
              type="checkbox"
              checked={settings.soundboard.global_hotkeys}
              onChange={(event) =>
                void saveSettings({
                  ...settings,
                  soundboard: {
                    ...settings.soundboard,
                    global_hotkeys: event.target.checked,
                  },
                })
              }
              className="h-4 w-4"
            />
          </Row>
          <Row label="Default category for new clips">
            <select
              value={settings.soundboard.default_category_id ?? ""}
              onChange={(event) =>
                void saveSettings({
                  ...settings,
                  soundboard: {
                    ...settings.soundboard,
                    default_category_id: event.target.value ? Number(event.target.value) : null,
                  },
                })
              }
              className="rounded-md border border-slate-600 bg-slate-950 px-2 py-1.5"
            >
              <option value="">No category</option>
              {categories.map((category) => (
                <option key={category.id} value={category.id}>
                  {category.name}
                </option>
              ))}
            </select>
          </Row>
        </Section>

        <Section title="Appearance">
          <Row label="Button size">
            <select
              value={settings.appearance.button_size}
              onChange={(event) =>
                void saveSettings({
                  ...settings,
                  appearance: { ...settings.appearance, button_size: event.target.value },
                })
              }
              className="rounded-md border border-slate-600 bg-slate-950 px-2 py-1.5"
            >
              <option value="compact">Compact</option>
              <option value="large">Large</option>
            </select>
          </Row>
        </Section>

        <Section title="Startup">
          <Row label="Start audio engine automatically">
            <input
              type="checkbox"
              checked={settings.startup.start_audio_engine}
              onChange={(event) =>
                void saveSettings({
                  ...settings,
                  startup: { start_audio_engine: event.target.checked },
                })
              }
              className="h-4 w-4"
            />
          </Row>
        </Section>
      </div>
    </div>
  );
}
