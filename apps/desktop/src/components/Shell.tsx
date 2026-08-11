import { Link } from "@tanstack/react-router";
import { useEffect, type ReactNode } from "react";

import { useAudioStore } from "../stores/audioStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useSoundboardStore } from "../stores/soundboardStore";
import { StatusBar } from "./StatusBar";

export function Shell({ children }: { children: ReactNode }) {
  useEffect(() => {
    void useAudioStore.getState().init();
    void useSoundboardStore.getState().init();
    void useSettingsStore.getState().load();
  }, []);

  return (
    <div className="flex h-full flex-col bg-slate-950 text-slate-100">
      <header className="flex items-center justify-between border-b border-slate-800 px-4 py-2">
        <div className="flex items-center gap-6">
          <h1 className="text-lg font-bold tracking-wide">ATCS SOUNDBOARD</h1>
          <nav className="flex items-center gap-3 text-sm">
            <Link
              to="/"
              className="rounded px-2 py-1 text-slate-300 hover:bg-slate-800 hover:text-white"
              activeProps={{ className: "bg-slate-800 text-white font-semibold" }}
            >
              Soundboard
            </Link>
            <Link
              to="/settings"
              className="rounded px-2 py-1 text-slate-300 hover:bg-slate-800 hover:text-white"
              activeProps={{ className: "bg-slate-800 text-white font-semibold" }}
            >
              Settings
            </Link>
          </nav>
        </div>
      </header>
      <main className="min-h-0 flex-1 overflow-hidden">{children}</main>
      <StatusBar />
    </div>
  );
}
