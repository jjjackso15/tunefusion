# Changelog

All notable changes to TuneFusion will be documented in this file.

## [Unreleased]

### Added

- **Pitch contour analysis** — pYIN-based F0 estimation via the `pyin` crate (pipeline `pitch_contour@0.1`)
  - Detects vocal pitch (65 Hz–1047 Hz range, C2–C6)
  - Outputs per-frame: timestamp, frequency (Hz), voiced flag, voicing probability
  - Unvoiced frames represented as `null` in JSON for clean serialization
  - New Tauri command: `analyze_pitch_contour`

- **Generalized artifact envelope** — `ArtifactEnvelope` now supports multiple artifact types via `ArtifactPayload` enum with `#[serde(flatten)]` + `#[serde(untagged)]`
  - Adding future artifact kinds (chords, tempo map, etc.) requires only a new enum variant

- **UI pitch stats** — "Analyze Pitch" runs alongside waveform analysis; displays voiced frame count, percentage, and mean frequency

### Changed

- **Analysis crate refactored into modules** — `lib.rs` (shared envelope + helpers), `waveform.rs`, `pitch_contour.rs`
- **Tauri upgraded to v2** — updated `tauri`, `tauri-build`, `@tauri-apps/api`, `tauri.conf.json` schema
- **File dialog uses native Tauri plugin** — replaced `<input type="file">` with `@tauri-apps/plugin-dialog` for proper filesystem paths
- **Both analyses run in parallel** — waveform + pitch contour invoked concurrently via `Promise.all`

### Dependencies

- Added `pyin = "1"` and `ndarray = "0.16"` to `crates/analysis`
- Added `serde_json = "1"` and `tauri-plugin-dialog = "2"` to desktop app

---

## [0.0.1] — 2026-03-06

### Added

- Initial project scaffold (Tauri + React + Rust workspace)
- Audio decoding via Symphonia (MP3, WAV, FLAC, OGG/Vorbis)
- Waveform peaks analysis (pipeline `waveform_peaks@0.1`, 256 buckets)
- Artifact-first pipeline with versioned JSON envelopes
- SVG waveform visualization in desktop UI
- Project documentation: architecture, requirements, roadmap, data model, ADRs
