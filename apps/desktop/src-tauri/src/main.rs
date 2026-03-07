#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

use audio_engine::decode_to_pcm;
use rusqlite::{params, Connection};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    artifacts_root: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
struct TrackRecord {
    id: String,
    title: String,
    audio_path: String,
    audio_hash: String,
    sample_rate: u32,
    duration_seconds: f64,
    created_at: String,
}

fn app_data_dir(app_handle: &AppHandle) -> Result<PathBuf, String> {
    app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))
}

fn open_db(db_path: &Path) -> Result<Connection, String> {
    let conn = Connection::open(db_path)
        .map_err(|e| format!("failed to open sqlite database ({}): {e}", db_path.display()))?;

    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS tracks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            audio_path TEXT NOT NULL,
            audio_hash TEXT NOT NULL,
            sample_rate INTEGER NOT NULL,
            duration_seconds REAL NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS analysis_runs (
            id TEXT PRIMARY KEY,
            track_id TEXT NOT NULL,
            pipeline_version TEXT NOT NULL,
            params_hash TEXT NOT NULL,
            audio_hash TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (track_id) REFERENCES tracks (id)
        );

        CREATE TABLE IF NOT EXISTS artifacts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            analysis_run_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            schema_version TEXT NOT NULL,
            file_path TEXT NOT NULL,
            sample_rate INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (analysis_run_id) REFERENCES analysis_runs (id)
        );

        CREATE INDEX IF NOT EXISTS idx_analysis_runs_track_id
            ON analysis_runs(track_id);

        CREATE INDEX IF NOT EXISTS idx_artifacts_run_id
            ON artifacts(analysis_run_id);
        ",
    )
    .map_err(|e| format!("failed to run migrations: {e}"))?;

    Ok(conn)
}

fn load_track(conn: &Connection, track_id: &str) -> Result<TrackRecord, String> {
    let mut stmt = conn
        .prepare(
            "
            SELECT id, title, audio_path, audio_hash, sample_rate, duration_seconds, created_at
            FROM tracks
            WHERE id = ?1
            ",
        )
        .map_err(|e| format!("failed preparing track query: {e}"))?;

    stmt.query_row(params![track_id], |row| {
        Ok(TrackRecord {
            id: row.get(0)?,
            title: row.get(1)?,
            audio_path: row.get(2)?,
            audio_hash: row.get(3)?,
            sample_rate: row.get::<_, u32>(4)?,
            duration_seconds: row.get(5)?,
            created_at: row.get(6)?,
        })
    })
    .map_err(|e| format!("track not found or unreadable: {e}"))
}

#[tauri::command]
fn import_track(audio_path: String, state: State<'_, AppState>) -> Result<TrackRecord, String> {
    let audio_path_ref = Path::new(&audio_path);
    if !audio_path_ref.exists() {
        return Err(format!("audio file does not exist: {}", audio_path_ref.display()));
    }

    let pcm = decode_to_pcm(audio_path_ref).map_err(|e| e.to_string())?;
    let duration_seconds =
        (pcm.frames.len() as f64) / (pcm.channels as f64) / (pcm.sample_rate as f64);
    let audio_hash = analysis::sha256_file(audio_path_ref).map_err(|e| e.to_string())?;

    let created_at = chrono::Utc::now().to_rfc3339();
    let track = TrackRecord {
        id: uuid::Uuid::new_v4().to_string(),
        title: audio_path_ref
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("untitled")
            .to_string(),
        audio_path: audio_path.clone(),
        audio_hash,
        sample_rate: pcm.sample_rate,
        duration_seconds,
        created_at,
    };

    let conn = open_db(&state.db_path)?;
    conn.execute(
        "
        INSERT INTO tracks (id, title, audio_path, audio_hash, sample_rate, duration_seconds, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ",
        params![
            track.id,
            track.title,
            track.audio_path,
            track.audio_hash,
            track.sample_rate,
            track.duration_seconds,
            track.created_at,
        ],
    )
    .map_err(|e| format!("failed to insert track: {e}"))?;

    Ok(track)
}

#[tauri::command]
fn list_tracks(state: State<'_, AppState>) -> Result<Vec<TrackRecord>, String> {
    let conn = open_db(&state.db_path)?;
    let mut stmt = conn
        .prepare(
            "
            SELECT id, title, audio_path, audio_hash, sample_rate, duration_seconds, created_at
            FROM tracks
            ORDER BY created_at DESC
            ",
        )
        .map_err(|e| format!("failed preparing list tracks query: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(TrackRecord {
                id: row.get(0)?,
                title: row.get(1)?,
                audio_path: row.get(2)?,
                audio_hash: row.get(3)?,
                sample_rate: row.get::<_, u32>(4)?,
                duration_seconds: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| format!("failed to query tracks: {e}"))?;

    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("failed reading tracks: {e}"))
}

#[tauri::command]
fn analyze_audio_file(
    track_id: String,
    state: State<'_, AppState>,
) -> Result<analysis::ArtifactEnvelope, String> {
    let conn = open_db(&state.db_path)?;
    let track = load_track(&conn, &track_id)?;

    let envelope =
        analysis::analyze_waveform_to_artifact_json(Path::new(&track.audio_path), &state.artifacts_root)
            .map_err(|e| e.to_string())?;

    let artifact_path = state
        .artifacts_root
        .join("analysis_runs")
        .join(&envelope.run_id)
        .join("waveform_peaks.json")
        .to_string_lossy()
        .to_string();

    conn.execute(
        "
        INSERT INTO analysis_runs (id, track_id, pipeline_version, params_hash, audio_hash, status, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, 'success', ?6)
        ",
        params![
            envelope.run_id,
            track_id,
            envelope.pipeline_version,
            envelope.params_hash,
            envelope.audio_hash,
            envelope.created_at,
        ],
    )
    .map_err(|e| format!("failed to persist analysis run: {e}"))?;

    conn.execute(
        "
        INSERT INTO artifacts (analysis_run_id, kind, schema_version, file_path, sample_rate, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ",
        params![
            envelope.run_id,
            envelope.kind,
            envelope.schema_version,
            artifact_path,
            envelope.sample_rate,
            envelope.created_at,
        ],
    )
    .map_err(|e| format!("failed to persist artifact metadata: {e}"))?;

    Ok(envelope)
}

#[tauri::command]
fn analyze_pitch_contour(
    track_id: String,
    state: State<'_, AppState>,
) -> Result<analysis::ArtifactEnvelope, String> {
    let conn = open_db(&state.db_path)?;
    let track = load_track(&conn, &track_id)?;

    let envelope = analysis::analyze_pitch_contour_to_artifact_json(
        Path::new(&track.audio_path),
        &state.artifacts_root,
    )
    .map_err(|e| e.to_string())?;

    let artifact_path = state
        .artifacts_root
        .join("analysis_runs")
        .join(&envelope.run_id)
        .join("pitch_contour.json")
        .to_string_lossy()
        .to_string();

    conn.execute(
        "
        INSERT INTO analysis_runs (id, track_id, pipeline_version, params_hash, audio_hash, status, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, 'success', ?6)
        ",
        params![
            envelope.run_id,
            track_id,
            envelope.pipeline_version,
            envelope.params_hash,
            envelope.audio_hash,
            envelope.created_at,
        ],
    )
    .map_err(|e| format!("failed to persist analysis run: {e}"))?;

    conn.execute(
        "
        INSERT INTO artifacts (analysis_run_id, kind, schema_version, file_path, sample_rate, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ",
        params![
            envelope.run_id,
            envelope.kind,
            envelope.schema_version,
            artifact_path,
            envelope.sample_rate,
            envelope.created_at,
        ],
    )
    .map_err(|e| format!("failed to persist artifact metadata: {e}"))?;

    Ok(envelope)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let data_dir = app_data_dir(&app_handle)?;
            std::fs::create_dir_all(&data_dir)
                .map_err(|e| format!("failed creating app data dir ({}): {e}", data_dir.display()))?;

            let db_path = data_dir.join("tunefusion.sqlite");
            let artifacts_root = data_dir.join("artifacts");
            std::fs::create_dir_all(&artifacts_root).map_err(|e| {
                format!(
                    "failed creating artifacts root ({}): {e}",
                    artifacts_root.display()
                )
            })?;

            open_db(&db_path)?;

            app.manage(AppState {
                db_path,
                artifacts_root,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            import_track,
            list_tracks,
            analyze_audio_file,
            analyze_pitch_contour
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
