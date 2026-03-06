# ADR-0003: Local-first storage model

Status: Accepted  
Date: 2026-03-05

## Context

TuneFusion must work offline and handle large audio assets.

## Decision

Metadata stored in SQLite.

Binary assets stored on disk.

Structure:

app_data/
  projects/
    <projectId>/
      audio/
      artifacts/
      sessions/

## Rationale

SQLite provides:

- reliability
- simple migrations
- fast queries

Filesystem storage allows large artifact files without DB bloat.

## Alternatives

Cloud-first architecture
Embedded binary storage in database

Both rejected for MVP complexity.