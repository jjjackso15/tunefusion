# ADR-0002: Artifact-first analysis pipeline

Status: Accepted  
Date: 2026-03-05

## Context

Audio analysis can be computationally expensive and difficult to perform
reliably in realtime.

TuneFusion must provide repeatable results and quick UI loading.

## Decision

Audio analysis will produce versioned artifacts.

Pipeline:

Import audio
→ run analysis
→ generate artifacts
→ UI reads artifacts

Artifacts include:

- waveform_peaks
- tempo_map
- beat_grid
- pitch_contour
- sections
- chords
- practice_loops
- score_report

Each run is stored as an analysis_run with:

- pipelineVersion
- parameters
- status
- artifact references

## Rationale

Benefits:

- deterministic analysis
- cached results
- faster UI
- easier debugging
- enables AI coaching

## Alternatives Considered

Realtime-only analysis.

This was rejected due to instability and complexity.

## Consequences

Pros:

- reliable results
- easy reprocessing
- scalable pipeline

Cons:

- requires disk storage for artifacts