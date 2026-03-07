# TuneFusion — Architecture (Desktop-first)

## 1) Overview
TuneFusion is a **desktop-first** practice app built with:
- **Tauri** (Rust backend + desktop shell)
- **React + TypeScript** (UI)
- **SQLite + filesystem** (local-first persistence)

The core design is **artifact-first**:
> Import audio → run analysis pipeline → write versioned artifacts → UI/coach consumes artifacts.

This keeps the UI simple, enables caching, and makes analysis reproducible.

## 2) Artifact-first analysis design
### 2.1 Inputs
- Source audio file (user-provided, stored locally)
- Pipeline parameters (JSON)
- `pipelineVersion` (string, semver-like)

### 2.2 Outputs (artifacts)
Artifacts are immutable outputs produced by a completed analysis run. Examples:
- `waveform_peaks`
- `tempo_map`, `beat_grid`
- `pitch_contour`
- `sections`
- `chords`
- `score_report` (post-session)

Artifacts are stored as files on disk with metadata in SQLite.

### 2.3 Versioned pipeline + caching
Each analysis run is uniquely identified by:
- `trackId`
- `pipelineVersion`
- `paramsHash` (hash of normalized params JSON)
- `inputHash` (hash of the source audio bytes, or stable content hash)

Caching rules:
- If there is a prior `analysis_run` with the same `trackId + pipelineVersion + paramsHash + inputHash` and status `success`, **reuse its artifacts**.
- If `pipelineVersion` changes, create a new run and keep old runs intact.
- If params change, create a new run.

This supports reproducibility and “upgrade analysis without breaking old projects”.

### 2.4 Analysis run states
- `queued` → `running` → (`success` | `failed` | `canceled`)
- Partial artifacts may exist for debugging, but UI should only use artifacts from `success`.

## 3) Module boundaries
### 3.1 `apps/desktop` (React UI)
- Import UI (select audio, create project/track)
- Practice UI (timeline, pitch lanes, chord prompts)
- Session UI (recording, score breakdown)
- Talks to Rust via Tauri commands/events

### 3.2 Rust core (Tauri backend)
- Owns local storage paths + SQLite connection
- Starts analysis jobs and reports progress
- Provides typed APIs to UI (list projects/tracks, fetch artifacts)

### 3.3 `crates/analysis` (analysis jobs)
- Orchestrates pipeline steps
- Produces artifacts + writes metadata
- Must be deterministic given the same input + params + version
- Modular: `lib.rs` (shared envelope + helpers), `waveform.rs`, `pitch_contour.rs`
- Uses `ArtifactPayload` enum to support multiple artifact types with a shared `ArtifactEnvelope`

Currently implemented artifact types:
- `waveform_peaks` (pipeline `waveform_peaks@0.1`) — 256-bucket peak magnitudes
- `pitch_contour` (pipeline `pitch_contour@0.1`) — pYIN F0 estimation + voicing confidence

### 3.4 `crates/audio_engine` (playback + timing)
- Playback transport + synchronization clock
- (Later) time-stretch for practice loops

### 3.5 Storage layer
- SQLite schema for metadata
- Filesystem layout for large artifacts and audio

### 3.6 Artifacts cache
- Disk-based, content-addressable-ish by (runId/kind) with hashes
- Indexed by SQLite for lookup and migration

### 3.7 Coach layer (future)
- Reads artifacts + session performance
- Generates coaching hints, loop suggestions, and DJ-style narration cues

## 4) Platform-Specific Storage Paths (Target State)
The following describes the **target state** (not fully implemented yet).

TuneFusion will follow the **XDG Base Directory Specification** (and equivalents on other platforms) via Tauri:
- **Metadata:** `tunefusion.sqlite` in `appDataDir`.
- **Artifacts:** `appDataDir/projects/<projectId>/...`
- **Cache:** `appCacheDir/analysis_runs/...`

Commands must always use `tauri::AppHandle` to resolve paths rather than hardcoding local relative paths.

Current state (as of 2026-03-07): `apps/desktop/src-tauri/src/main.rs` still passes a relative `artifacts` path and needs migration to `appDataDir`.

## 5) Type Safety & Synchronization (Target State)
The following is also a **target-state rule** that will be enforced as type generation is added:

To prevent drift between Rust structs (backend) and TypeScript interfaces (frontend):
- **Tooling:** Use `ts-rs` or `specta` to automatically generate TypeScript definitions from Rust source files during build or on demand.
- **Location:** Generated types should be written to `apps/desktop/src/types/generated/`.
- **Rule:** Do **not** manually define `ArtifactEnvelope` or `ArtifactPayload` types in React; always import from the generated definitions.

## 6) Minimal sequence diagram

```text
User            UI (React)           Rust Core (Tauri)        Analysis Pipeline        Storage (SQLite+FS)
 |                 |                        |                      |                          |
 | Import audio    |                        |                      |                          |
 |---------------->| create project/track   |                      |                          |
 |                 |----------------------->| persist track        |------------------------->| write track + audio
 |                 |                        | enqueue analysis     |                          |
 |                 |                        |--------------------->| run pipeline             |
 |                 |                        |                      |------------------------->| write artifacts + metadata
 |                 | progress events        |<---------------------|                          |
 | Practice        | request artifacts       |                      |                          |
 |---------------->|----------------------->| load artifact index   |------------------------->| read metadata + files
 |                 | render timeline        |                      |                          |
 | Perform         | stream mic input (later)| scoring (later)      |                          |
 |---------------->| persist session+score  |--------------------->|                          |
 |                 |                        |------------------------->| write session + score
```

## 7) Demo Artifact Policy
- Runtime-generated analysis outputs under `apps/desktop/src-tauri/artifacts/analysis_runs/` are treated as ephemeral and are git-ignored.
- Curated demo artifacts are allowed in git and should be stored under `apps/desktop/src-tauri/artifacts/demo/`.
- Demo artifacts must be deterministic snapshots intended for docs, test fixtures, or product demos; avoid committing machine-specific scratch output.
