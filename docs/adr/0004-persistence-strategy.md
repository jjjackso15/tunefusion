# ADR 0004: Persistence Strategy (SQLite + File Storage)

## Status
Proposed (v0.1)

## Context
TuneFusion requires a local-first persistence layer to manage project metadata, track analysis runs, and track artifact locations. While some artifacts are large JSON or binary files, the metadata must be queryable and relational.

## Decision
- **Metadata Storage:** Use **SQLite** for all relational data (projects, tracks, analysis runs, artifact index).
- **Library Selection:** We will use **`rusqlite`** for simplicity in our synchronous Tauri commands, or **`sqlx`** if async features (like pooling or compile-time query checking) are required. For MVP, `rusqlite` is the baseline.
- **Migrations:** Use a simple migration runner (e.g., `rusqlite_migration`) to handle schema evolution.
- **Binary/Large Data:** Store large artifacts on the filesystem under the Tauri `appDataDir`. Do **not** store large blobs (audio, waveforms, pitch contours) inside SQLite.
- **Project Portability:** Metadata should be stored in a central `tunefusion.sqlite` database in the user's data directory, referencing absolute paths or project-relative paths.

## Consequences
- **Robustness:** Using SQLite ensures ACID compliance for metadata updates.
- **Path Management:** We must use Tauri's path resolver to ensure portability across Linux, macOS, and Windows.
- **Version Tracking:** `analysis_runs` will serve as the primary lookup for which artifacts are valid for a given track and pipeline version.
- **Data Integrity:** The `artifacts` table must store hashes (SHA-256) of the files on disk to detect corruption or manual edits.
