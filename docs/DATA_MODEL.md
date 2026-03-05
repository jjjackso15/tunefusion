# TuneFusion — Data Model (Local-first)

TuneFusion uses **SQLite for metadata** and the **filesystem for large blobs** (audio + artifacts).

## 1) Artifact kinds
The analysis pipeline can produce these artifact kinds (initial list):
- `waveform_peaks`
- `tempo_map`
- `beat_grid`
- `pitch_contour`
- `sections`
- `chords`
- `practice_loops`
- `score_report`

## 2) Minimal SQLite schema (proposal)

> Note: IDs shown as TEXT (UUID). Timestamps are ISO strings or integer ms.

### `projects`
- `id` TEXT PRIMARY KEY
- `name` TEXT NOT NULL
- `created_at` TEXT NOT NULL
- `updated_at` TEXT NOT NULL

### `tracks`
- `id` TEXT PRIMARY KEY
- `project_id` TEXT NOT NULL REFERENCES projects(id)
- `title` TEXT NOT NULL
- `artist` TEXT
- `duration_ms` INTEGER
- `source_path` TEXT NOT NULL  -- path to imported audio (within project folder)
- `input_hash` TEXT NOT NULL   -- stable hash of source audio
- `created_at` TEXT NOT NULL

INDEX: `(project_id)`

### `analysis_runs`
- `id` TEXT PRIMARY KEY
- `track_id` TEXT NOT NULL REFERENCES tracks(id)
- `pipeline_version` TEXT NOT NULL
- `params_json` TEXT NOT NULL
- `params_hash` TEXT NOT NULL
- `input_hash` TEXT NOT NULL
- `status` TEXT NOT NULL CHECK(status IN ('queued','running','success','failed','canceled'))
- `started_at` TEXT
- `finished_at` TEXT
- `error_message` TEXT

UNIQUE (cache key): `(track_id, pipeline_version, params_hash, input_hash)`

### `artifacts`
- `id` TEXT PRIMARY KEY
- `analysis_run_id` TEXT NOT NULL REFERENCES analysis_runs(id)
- `kind` TEXT NOT NULL
- `path` TEXT NOT NULL          -- relative path under project folder
- `content_hash` TEXT
- `format` TEXT NOT NULL        -- e.g., json, msgpack, npy, wav
- `format_version` INTEGER NOT NULL DEFAULT 1
- `bytes` INTEGER
- `created_at` TEXT NOT NULL

INDEX: `(analysis_run_id, kind)`

### `practice_sessions`
- `id` TEXT PRIMARY KEY
- `project_id` TEXT NOT NULL REFERENCES projects(id)
- `track_id` TEXT NOT NULL REFERENCES tracks(id)
- `user_profile` TEXT NOT NULL  -- MVP: a display name (local-only)
- `mode` TEXT NOT NULL          -- vocal | guitar | both
- `started_at` TEXT NOT NULL
- `ended_at` TEXT
- `notes` TEXT

INDEX: `(track_id, started_at)`

### `scores`
- `id` TEXT PRIMARY KEY
- `practice_session_id` TEXT NOT NULL REFERENCES practice_sessions(id)
- `overall` REAL NOT NULL
- `pitch` REAL
- `timing` REAL
- `streak_best` INTEGER
- `details_artifact_id` TEXT REFERENCES artifacts(id)  -- optional score_report link
- `created_at` TEXT NOT NULL

## 3) Folder layout (per project)
All paths below are relative to the app data root:

```text
app_data/
  projects/
    <projectId>/
      project.sqlite
      tracks/
        <trackId>/
          source/
            original.<ext>
          artifacts/
            <analysisRunId>/
              waveform_peaks.json
              tempo_map.json
              beat_grid.json
              pitch_contour.json
              sections.json
              chords.json
              practice_loops.json
      sessions/
        <sessionId>/
          recordings/
            vocal.wav            (optional)
          score_report.json      (optional)
```

Design notes:
- Keep artifacts immutable: new `analysis_run` → new artifact folder.
- UI picks the best run (usually latest `pipeline_version`) but old runs remain usable.
