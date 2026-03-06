# ADR-0001: Desktop-first architecture

Status: Accepted  
Date: 2026-03-05

## Context

TuneFusion requires reliable audio playback, looping, waveform analysis,
and low-latency interaction with local files.

Web browsers introduce limitations around:

- filesystem access
- audio latency
- offline workflows
- large audio file processing

## Decision

TuneFusion will be a desktop application.

Primary stack:

Tauri + React + Rust backend

## Rationale

Desktop provides:

- direct filesystem access
- deterministic performance
- easier audio processing
- offline usability

## Alternatives Considered

Web-only application
Electron desktop app

## Consequences

Pros:

- better audio performance
- simpler storage model
- offline-first capability

Cons:

- packaging required for multiple OS targets

## When to Revisit

If a cloud collaboration feature becomes the primary product experience.