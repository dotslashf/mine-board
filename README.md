# ATCS Soundboard

A local-first soundboard for ATC simulation: combine a physical microphone
with one-click sound clips into a single **virtual microphone** that
applications (VATSIM clients, Discord, etc.) can capture from.

## Architecture

Bun monorepo with two parts:

- `apps/desktop` — React 19 + TanStack Router + Zustand + Tailwind v4 UI,
  served by a Bun dev server (port 1420).
- `src-tauri/` — Rust backend split into two crates:
  - `crates/core` — headless library (SQLite, WAV decode/resample, RT-safe
    mixer & clip scheduler). No Tauri/PipeWire deps; runs tests anywhere.
  - `crates/app` — Tauri shell: PipeWire audio engine (virtual mic + mic
    capture + optional monitor), global hotkeys, Tauri commands.

## Requirements

- Linux with PipeWire (the audio engine is Linux/PipeWire-only)
- [Bun](https://bun.com) >= 1.2
- Rust toolchain for the Tauri shell

## Dev

```bash
bun install

# Web UI only (React + Bun dev server on :1420)
bun run dev --filter @atcs/desktop

# Desktop app (Tauri window + PipeWire engine)
bun run tauri dev

# Regenerate the app icons
bun run icons
```

## Tests & checks

```bash
bun test                      # workspace (Rust core + React UI)
bun run typecheck             # TS + Rust (cargo check)
bun run build                 # production build
bun run tauri build           # deb/appimage bundle
```

## Releases

Push a version tag and CI builds and uploads both bundles to a GitHub
Release (draft):

```bash
git tag v0.1.1
git push origin v0.1.1
```

Then publish the draft release on GitHub. The local flow is the same as
CI (`bun run tauri build`); bundles land in
`src-tauri/target/release/bundle/{deb,appimage}/`.

## Notes

- Only **WAV** files are supported today; the file picker filters to `.wav`
  and imports are validated at import time.
- The engine's real-time callbacks stay allocation-free; control-plane
  changes are sent over a bounded command channel.
