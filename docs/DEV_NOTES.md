# TuneFusion — Dev Notes

## 1) Prerequisites (planned)
- Rust toolchain (stable)
- Node.js (LTS) + package manager (pnpm recommended)
- Tauri prerequisites for Linux (webkit2gtk, etc.)

## 2) Repo layout
- `apps/desktop/` — Tauri + React + TypeScript UI
- `crates/analysis/` — analysis pipeline + artifact generation
- `crates/audio_engine/` — audio playback + timing utilities
- `packages/shared/` — shared TS types/schemas

## 3) Common commands (placeholders)
From repo root:

```bash
# install JS deps (when workspace is created)
pnpm install

# run desktop app (when Tauri app exists)
pnpm -C apps/desktop dev

# run Rust tests
cargo test
```

## 4) Troubleshooting (placeholders)
- If Tauri build fails on Linux, confirm system deps for webkit2gtk are installed.
- If audio decoding fails, verify you have the codec support you expect (FFmpeg vs pure Rust crates).

## 5) Re-running analysis with a new pipelineVersion
Principle: **never mutate old artifacts in place**.

When you bump analysis logic in a way that changes outputs:
1. Increment `pipelineVersion` (e.g., `0.1.0` → `0.1.1`).
2. New analyses for a track create a new row in `analysis_runs`.
3. Artifacts are written under:
   `projects/<projectId>/tracks/<trackId>/artifacts/<analysisRunId>/...`
4. UI selects an analysis run to use (default: latest successful run).

This ensures:
- Older projects remain playable.
- You can compare outputs between versions.
- No cache collisions.
