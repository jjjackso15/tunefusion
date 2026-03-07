# TuneFusion — Roadmap

## Milestone v0.1 (MVP)
### Goals
- Desktop app skeleton (Tauri + React)
- Import local audio (MP3/WAV)
- Run analysis pipeline (even if imperfect) to produce at least:
  - waveform peaks
  - pitch contour
  - chord timeline
- Practice view reads artifacts and renders:
  - timeline + waveform
  - pitch lane overlay
  - chord prompts
- Local profiles (simple name-based)
- Persist sessions + a basic score

### Progress
- [x] Tauri 2 + React scaffold with native file dialog
- [x] Audio decoding (MP3, WAV, FLAC, OGG) via Symphonia
- [x] Waveform peaks artifact (`waveform_peaks@0.1`)
- [x] Pitch contour artifact (`pitch_contour@0.1`) via pYIN
- [x] Generalized `ArtifactEnvelope` supporting multiple artifact types
- [x] UI displays waveform + pitch stats
- [ ] Chord timeline detection
- [ ] Practice view (pitch lanes, chord prompts)
- [ ] Local profiles
- [ ] Session persistence + scoring

### Non-goals
- Cloud sync
- Public sharing
- Perfect transcription accuracy

### Acceptance criteria
- A user can import a song, analyze it, practice, and see a stored score history.
- Re-running analysis with new `pipelineVersion` creates a new run without breaking old projects.

## Milestone v0.2 (Better analysis + coaching)
### Goals
- Improved analysis quality:
  - tempo map + beat grid
  - sections (verse/chorus markers if feasible)
- Coach layer (non-annoying):
  - detects trouble spots
  - suggests practice loops
  - optional DJ-style narration cues
- Better scoring breakdown and visualization

### Non-goals
- Real-time remote competition

### Acceptance criteria
- Coach suggestions are persisted and actionable (one-click loop + replay).
- Analysis pipeline is parameterized and versioned, with reproducible outputs.

## Milestone v0.3 (Competition + sync/export)
### Goals
- Local competition mode (two players)
- Project export/import (bundle audio + artifacts + sessions)
- Optional cloud sync (stretch):
  - accounts
  - leaderboard
  - privacy controls

### Non-goals
- Becoming a streaming platform

### Acceptance criteria
- Users can compete locally and compare results.
- A project can be exported on one machine and imported on another with no data loss.
