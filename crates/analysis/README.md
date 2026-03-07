# crates/analysis

Rust crate for TuneFusion's **artifact-first** analysis pipeline.

## Implemented artifact types
- **`waveform_peaks`** (`waveform.rs`) — peak magnitude per bucket across the track (256 buckets, pipeline `waveform_peaks@0.1`)
- **`pitch_contour`** (`pitch_contour.rs`) — pYIN pitch detection with voicing confidence (pipeline `pitch_contour@0.1`, uses the `pyin` crate)

## Architecture
- `lib.rs` — shared `ArtifactEnvelope` + `ArtifactPayload` enum, hashing/IO helpers
- `waveform.rs` — waveform peak computation + artifact generation
- `pitch_contour.rs` — pYIN pitch detection + artifact generation

Each analysis produces a versioned JSON artifact under `artifacts/analysis_runs/<run_id>/`.

## Planned
- Chord timeline detection
- Tempo map + beat grid
- Section markers
