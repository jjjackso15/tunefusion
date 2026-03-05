# TuneFusion — Guardrails (Do Not Drift)

These are the non-negotiables for this project. Keep them consistent across docs, issues, and implementation.

## Product + Platform
- **Desktop-first** product.
- Target stack: **Tauri + React (TypeScript) + Rust backend**.
- Do **not** pivot to web-only.

## Architecture
- **Artifact-first**, not realtime-first.
- Analysis produces **cached artifacts** on disk.
- Every analysis run is keyed by:
  - `pipelineVersion`
  - normalized `params` (with `paramsHash`)
  - `input_hash` for the source audio
- **Never overwrite old artifacts in place.**
  - New pipeline versions or params → new analysis run → new artifact folder.

## Storage model
- **SQLite stores metadata** (projects, tracks, runs, artifact index, sessions, scores).
- **Binaries/blobs live on disk** (audio + artifact files) and should be hash-addressed or hash-verified.

## MVP focus (v0.1)
Keep v0.1 narrowly focused:
1. Import audio
2. Analyze (tempo/beat grid at minimum)
3. Practice loop
4. Basic score
5. Persist everything locally

## Avoid premature complexity
- No cloud.
- No accounts.
- No realtime multi-user.
- No “perfect” transcription goals in MVP.

## Where this is enforced
- `docs/ARCHITECTURE.md`
- `docs/DATA_MODEL.md`
- Milestones and issues (v0.1/v0.2/v0.3)
