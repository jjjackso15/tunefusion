# TuneFusion

TuneFusion is a **desktop-first** practice app concept that fuses **vocals + guitar** around real songs.

Core idea: **artifact-first, local-first**.
- You import audio.
- A versioned analysis pipeline generates cached artifacts (pitch contour, chords, beat grid, etc.).
- The UI + coach layer read artifacts (not raw audio) to drive practice, scoring, and hints.

## Development
AI agents and contributors should read **AGENT_GUIDELINES.md** before making structural changes.

## Stack (planned)
- Desktop: **Tauri**
- UI: **React + TypeScript**
- Backend: **Rust**
- Storage: **SQLite + filesystem artifacts**

## Docs
- Guardrails: `docs/GUARDRAILS.md`
- Architecture: `docs/ARCHITECTURE.md`
- Data model + folder layout: `docs/DATA_MODEL.md`
- Roadmap: `docs/ROADMAP.md`
- Dev notes: `docs/DEV_NOTES.md`
- Product requirements: `docs/REQUIREMENTS.md`

## Dev quick-start (high level)
This repo is currently **docs + scaffold only**.

Once the workspace is wired up:
- `pnpm install`
- `pnpm -C apps/desktop dev`

## Local-first note
TuneFusion is designed to work without cloud by default. Competition/sync is a future optional layer.
