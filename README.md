# TuneFusion

TuneFusion is a **desktop-first** practice app concept that fuses **vocals + guitar** around real songs.

Core idea: **artifact-first, local-first**.
- You import audio.
- A versioned analysis pipeline generates cached artifacts (pitch contour, chords, beat grid, etc.).
- The UI + coach layer read artifacts (not raw audio) to drive practice, scoring, and hints.

## Development
AI agents and contributors should read **AGENT_GUIDELINES.md** before making structural changes.

## Stack
- Desktop: **Tauri 2**
- UI: **React + TypeScript**
- Backend: **Rust**
- Storage: **SQLite + filesystem artifacts**

## Current status
The analysis pipeline produces two artifact types:
- **Waveform peaks** — peak magnitudes per bucket for waveform visualization
- **Pitch contour** — pYIN-based F0 estimation with voicing confidence

The desktop app can import audio (MP3/WAV/FLAC/OGG), run both analyses, and display results.

## Docs
- Guardrails: `docs/GUARDRAILS.md`
- Architecture: `docs/ARCHITECTURE.md`
- Data model + folder layout: `docs/DATA_MODEL.md`
- Roadmap: `docs/ROADMAP.md`
- Dev notes: `docs/DEV_NOTES.md`
- Product requirements: `docs/REQUIREMENTS.md`
- Release notes: `CHANGELOG.md`

## Dev quick-start
```bash
# install JS deps
pnpm install

# run desktop app
pnpm -C apps/desktop tauri dev

# run Rust tests
cargo test
```

## Local-first note
TuneFusion is designed to work without cloud by default. Competition/sync is a future optional layer.
