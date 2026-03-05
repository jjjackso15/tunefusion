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

## 4) Future extensions
- **Realtime coaching**: low-latency pitch detection during playback; overlay feedback in UI.
- **Multi-user competition**: local profiles first; later optional cloud sync.
- **Sync/export**: export project bundles (audio + artifacts + sessions) for sharing/backups.

## 5) Minimal sequence diagram

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
