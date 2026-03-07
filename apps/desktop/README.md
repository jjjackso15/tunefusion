# TuneFusion Desktop App (Tauri 2 + React + TypeScript)

Desktop application for TuneFusion.

## Stack
- **Tauri 2** (Rust backend + secure desktop shell)
- **React + TypeScript** (UI)
- Local-first storage (SQLite + filesystem)

## Features
- Import audio files (MP3, WAV, FLAC, OGG) via native file dialog
- Run waveform + pitch contour analysis in parallel
- Display waveform visualization (SVG)
- Display pitch contour statistics (voiced frames, mean frequency)

## Tauri commands
- `analyze_audio_file` — compute waveform peaks artifact
- `analyze_pitch_contour` — compute pitch contour artifact via pYIN

## Dev
```bash
pnpm install
pnpm tauri dev
```
