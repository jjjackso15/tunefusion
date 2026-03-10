#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use audio_engine::decode_to_pcm;
use crossbeam_channel::{bounded, Sender, Receiver};
use game_engine::{
    AudioPlayer, MicCapture, PitchDetector, PlaybackState,
    ScoringEngine,
};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Clone, Serialize)]
struct AnalysisProgress {
    phase: String,
    step: u32,
    total_steps: u32,
    message: String,
}

/// Game mode type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameMode {
    Solo,
    Competition,
}

/// Current game state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameState {
    Ready,
    Countdown,
    Playing,
    Paused,
    Finished,
}

/// Game state change event.
#[derive(Clone, Serialize)]
pub struct GameStateChange {
    pub state: GameState,
    pub countdown: Option<u32>,
}

/// Commands sent to the game thread.
#[derive(Debug)]
enum GameCommand {
    Start {
        audio_path: String,
        track_id: String,
        player_name: String,
        mode: GameMode,
        target_pitches: Vec<TargetPitch>,
    },
    BeginCountdown,
    Pause,
    Resume,
    Stop,
    Shutdown,
}

/// Target pitch data loaded from pitch contour artifact.
#[derive(Debug, Clone)]
struct TargetPitch {
    time_ms: u64,
    frequency_hz: Option<f64>,
    voiced: bool,
}

#[derive(Clone)]
struct AppState {
    db_path: PathBuf,
    artifacts_root: PathBuf,
}

/// Thread-safe game control state.
struct GameControlState {
    /// Channel to send commands to the game thread.
    command_tx: Sender<GameCommand>,
    /// Channel to receive results from the game thread.
    result_rx: Receiver<GameSessionResult>,
    /// Current game info (for returning results).
    current_game: Option<GameInfo>,
}

/// Game info stored in the control state.
#[derive(Debug, Clone)]
struct GameInfo {
    track_id: String,
    player_name: String,
    mode: GameMode,
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

        -- Game sessions table
        CREATE TABLE IF NOT EXISTS game_sessions (
            id TEXT PRIMARY KEY,
            track_id TEXT NOT NULL REFERENCES tracks(id),
            player_name TEXT NOT NULL,
            mode TEXT NOT NULL,
            score REAL NOT NULL,
            accuracy_pct REAL NOT NULL,
            perfect_count INTEGER NOT NULL,
            great_count INTEGER NOT NULL,
            good_count INTEGER NOT NULL,
            ok_count INTEGER NOT NULL,
            miss_count INTEGER NOT NULL,
            max_streak INTEGER NOT NULL,
            created_at TEXT NOT NULL
        );

        -- Leaderboards table
        CREATE TABLE IF NOT EXISTS leaderboards (
            id TEXT PRIMARY KEY,
            track_id TEXT NOT NULL REFERENCES tracks(id),
            player_name TEXT NOT NULL,
            score REAL NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_game_sessions_track_id
            ON game_sessions(track_id);

        CREATE INDEX IF NOT EXISTS idx_leaderboards_track_score
            ON leaderboards(track_id, score DESC);
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
async fn import_track(audio_path: String, state: State<'_, AppState>) -> Result<TrackRecord, String> {
    let db_path = state.db_path.clone();
    let audio_path_clone = audio_path.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let audio_path_ref = Path::new(&audio_path_clone);
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
            audio_path: audio_path_clone,
            audio_hash,
            sample_rate: pcm.sample_rate,
            duration_seconds,
            created_at,
        };

        let conn = open_db(&db_path)?;
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
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
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
async fn analyze_audio_file(
    track_id: String,
    state: State<'_, AppState>,
) -> Result<analysis::ArtifactEnvelope, String> {
    let db_path = state.db_path.clone();
    let artifacts_root = state.artifacts_root.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let conn = open_db(&db_path)?;
        let track = load_track(&conn, &track_id)?;

        let envelope =
            analysis::analyze_waveform_to_artifact_json(Path::new(&track.audio_path), &artifacts_root)
                .map_err(|e| e.to_string())?;

        let artifact_path = artifacts_root
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
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

#[tauri::command]
async fn analyze_pitch_contour(
    track_id: String,
    state: State<'_, AppState>,
) -> Result<analysis::ArtifactEnvelope, String> {
    let db_path = state.db_path.clone();
    let artifacts_root = state.artifacts_root.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let conn = open_db(&db_path)?;
        let track = load_track(&conn, &track_id)?;

        let envelope = analysis::analyze_pitch_contour_to_artifact_json(
            Path::new(&track.audio_path),
            &artifacts_root,
        )
        .map_err(|e| e.to_string())?;

        let artifact_path = artifacts_root
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
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

#[derive(Clone, Serialize)]
struct AnalysisResult {
    waveform: analysis::ArtifactEnvelope,
    pitch: analysis::ArtifactEnvelope,
}

#[tauri::command]
async fn analyze_track(
    track_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<AnalysisResult, String> {
    let db_path = state.db_path.clone();
    let artifacts_root = state.artifacts_root.clone();

    // Emit: Starting
    let _ = app_handle.emit(
        "analysis-progress",
        AnalysisProgress {
            phase: "starting".to_string(),
            step: 0,
            total_steps: 3,
            message: "Starting analysis...".to_string(),
        },
    );

    let app_handle_clone = app_handle.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let conn = open_db(&db_path)?;
        let track = load_track(&conn, &track_id)?;
        let audio_path = Path::new(&track.audio_path);

        // Phase 1: Waveform analysis (fast)
        let _ = app_handle_clone.emit(
            "analysis-progress",
            AnalysisProgress {
                phase: "waveform".to_string(),
                step: 1,
                total_steps: 3,
                message: "Analyzing waveform...".to_string(),
            },
        );

        let waveform_envelope =
            analysis::analyze_waveform_to_artifact_json(audio_path, &artifacts_root)
                .map_err(|e| e.to_string())?;

        let waveform_artifact_path = artifacts_root
            .join("analysis_runs")
            .join(&waveform_envelope.run_id)
            .join("waveform_peaks.json")
            .to_string_lossy()
            .to_string();

        conn.execute(
            "INSERT INTO analysis_runs (id, track_id, pipeline_version, params_hash, audio_hash, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'success', ?6)",
            params![
                waveform_envelope.run_id,
                track_id,
                waveform_envelope.pipeline_version,
                waveform_envelope.params_hash,
                waveform_envelope.audio_hash,
                waveform_envelope.created_at,
            ],
        )
        .map_err(|e| format!("failed to persist waveform analysis run: {e}"))?;

        conn.execute(
            "INSERT INTO artifacts (analysis_run_id, kind, schema_version, file_path, sample_rate, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                waveform_envelope.run_id,
                waveform_envelope.kind,
                waveform_envelope.schema_version,
                waveform_artifact_path,
                waveform_envelope.sample_rate,
                waveform_envelope.created_at,
            ],
        )
        .map_err(|e| format!("failed to persist waveform artifact: {e}"))?;

        // Phase 2: Pitch contour analysis (slow)
        let _ = app_handle_clone.emit(
            "analysis-progress",
            AnalysisProgress {
                phase: "pitch".to_string(),
                step: 2,
                total_steps: 3,
                message: "Analyzing pitch contour (this may take a while)...".to_string(),
            },
        );

        let pitch_envelope =
            analysis::analyze_pitch_contour_to_artifact_json(audio_path, &artifacts_root)
                .map_err(|e| e.to_string())?;

        let pitch_artifact_path = artifacts_root
            .join("analysis_runs")
            .join(&pitch_envelope.run_id)
            .join("pitch_contour.json")
            .to_string_lossy()
            .to_string();

        conn.execute(
            "INSERT INTO analysis_runs (id, track_id, pipeline_version, params_hash, audio_hash, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'success', ?6)",
            params![
                pitch_envelope.run_id,
                track_id,
                pitch_envelope.pipeline_version,
                pitch_envelope.params_hash,
                pitch_envelope.audio_hash,
                pitch_envelope.created_at,
            ],
        )
        .map_err(|e| format!("failed to persist pitch analysis run: {e}"))?;

        conn.execute(
            "INSERT INTO artifacts (analysis_run_id, kind, schema_version, file_path, sample_rate, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                pitch_envelope.run_id,
                pitch_envelope.kind,
                pitch_envelope.schema_version,
                pitch_artifact_path,
                pitch_envelope.sample_rate,
                pitch_envelope.created_at,
            ],
        )
        .map_err(|e| format!("failed to persist pitch artifact: {e}"))?;

        // Phase 3: Complete
        let _ = app_handle_clone.emit(
            "analysis-progress",
            AnalysisProgress {
                phase: "complete".to_string(),
                step: 3,
                total_steps: 3,
                message: "Analysis complete!".to_string(),
            },
        );

        Ok(AnalysisResult {
            waveform: waveform_envelope,
            pitch: pitch_envelope,
        })
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

// ============================================================================
// VOCAL ISOLATION & MIDI IMPORT COMMANDS
// ============================================================================

/// Check if Demucs is available for vocal isolation.
#[tauri::command]
fn check_demucs_available() -> bool {
    analysis::is_demucs_available()
}

/// Debug Demucs environment - returns detailed information about paths and detection.
#[derive(Clone, Serialize)]
struct DemucsDebugInfoResponse {
    home_dir: Option<String>,
    python_version: Option<String>,
    pythonpath_set: Option<String>,
    path_set: Option<String>,
    site_packages_found: Vec<String>,
    demucs_import_result: String,
    demucs_location: Option<String>,
}

#[tauri::command]
fn debug_demucs_environment() -> DemucsDebugInfoResponse {
    let info = analysis::debug_demucs_environment();
    DemucsDebugInfoResponse {
        home_dir: info.home_dir,
        python_version: info.python_version,
        pythonpath_set: info.pythonpath_set,
        path_set: info.path_set,
        site_packages_found: info.site_packages_found,
        demucs_import_result: info.demucs_import_result,
        demucs_location: info.demucs_location,
    }
}

/// Analyze track with vocal isolation using Demucs.
#[tauri::command]
async fn analyze_track_with_vocals(
    track_id: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<AnalysisResult, String> {
    let db_path = state.db_path.clone();
    let artifacts_root = state.artifacts_root.clone();

    let app_handle_clone = app_handle.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let conn = open_db(&db_path)?;
        let track = load_track(&conn, &track_id)?;
        let audio_path = Path::new(&track.audio_path);

        // Phase 1: Vocal isolation with Demucs
        let _ = app_handle_clone.emit(
            "analysis-progress",
            AnalysisProgress {
                phase: "vocals".to_string(),
                step: 1,
                total_steps: 4,
                message: "Isolating vocals with Demucs (this may take several minutes)...".to_string(),
            },
        );

        let vocal_config = analysis::VocalIsolationConfig {
            output_dir: artifacts_root.join("stems"),
            ..Default::default()
        };

        let vocal_result = analysis::isolate_vocals(audio_path, &vocal_config)
            .map_err(|e| e.to_string())?;

        // Phase 2: Waveform analysis
        let _ = app_handle_clone.emit(
            "analysis-progress",
            AnalysisProgress {
                phase: "waveform".to_string(),
                step: 2,
                total_steps: 4,
                message: "Analyzing waveform...".to_string(),
            },
        );

        let waveform_envelope =
            analysis::analyze_waveform_to_artifact_json(audio_path, &artifacts_root)
                .map_err(|e| e.to_string())?;

        let waveform_artifact_path = artifacts_root
            .join("analysis_runs")
            .join(&waveform_envelope.run_id)
            .join("waveform_peaks.json")
            .to_string_lossy()
            .to_string();

        conn.execute(
            "INSERT INTO analysis_runs (id, track_id, pipeline_version, params_hash, audio_hash, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'success', ?6)",
            params![
                waveform_envelope.run_id,
                track_id,
                waveform_envelope.pipeline_version,
                waveform_envelope.params_hash,
                waveform_envelope.audio_hash,
                waveform_envelope.created_at,
            ],
        )
        .map_err(|e| format!("failed to persist waveform analysis run: {e}"))?;

        conn.execute(
            "INSERT INTO artifacts (analysis_run_id, kind, schema_version, file_path, sample_rate, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                waveform_envelope.run_id,
                waveform_envelope.kind,
                waveform_envelope.schema_version,
                waveform_artifact_path,
                waveform_envelope.sample_rate,
                waveform_envelope.created_at,
            ],
        )
        .map_err(|e| format!("failed to persist waveform artifact: {e}"))?;

        // Phase 3: Pitch contour analysis ON VOCALS ONLY
        let _ = app_handle_clone.emit(
            "analysis-progress",
            AnalysisProgress {
                phase: "pitch".to_string(),
                step: 3,
                total_steps: 4,
                message: "Analyzing pitch contour from isolated vocals...".to_string(),
            },
        );

        // Analyze the isolated vocals instead of the full mix
        let pitch_envelope =
            analysis::analyze_pitch_contour_to_artifact_json(&vocal_result.vocals_path, &artifacts_root)
                .map_err(|e| e.to_string())?;

        let pitch_artifact_path = artifacts_root
            .join("analysis_runs")
            .join(&pitch_envelope.run_id)
            .join("pitch_contour.json")
            .to_string_lossy()
            .to_string();

        conn.execute(
            "INSERT INTO analysis_runs (id, track_id, pipeline_version, params_hash, audio_hash, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'success', ?6)",
            params![
                pitch_envelope.run_id,
                track_id,
                format!("{}_demucs", pitch_envelope.pipeline_version),
                pitch_envelope.params_hash,
                pitch_envelope.audio_hash,
                pitch_envelope.created_at,
            ],
        )
        .map_err(|e| format!("failed to persist pitch analysis run: {e}"))?;

        conn.execute(
            "INSERT INTO artifacts (analysis_run_id, kind, schema_version, file_path, sample_rate, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                pitch_envelope.run_id,
                pitch_envelope.kind,
                pitch_envelope.schema_version,
                pitch_artifact_path,
                pitch_envelope.sample_rate,
                pitch_envelope.created_at,
            ],
        )
        .map_err(|e| format!("failed to persist pitch artifact: {e}"))?;

        // Phase 4: Complete
        let _ = app_handle_clone.emit(
            "analysis-progress",
            AnalysisProgress {
                phase: "complete".to_string(),
                step: 4,
                total_steps: 4,
                message: "Analysis complete (with vocal isolation)!".to_string(),
            },
        );

        Ok(AnalysisResult {
            waveform: waveform_envelope,
            pitch: pitch_envelope,
        })
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

/// MIDI track info for listing.
#[derive(Debug, Clone, Serialize)]
pub struct MidiTrackInfo {
    pub index: usize,
    pub name: String,
    pub note_count: usize,
}

/// List tracks in a MIDI file.
#[tauri::command]
fn list_midi_tracks(midi_path: String) -> Result<Vec<MidiTrackInfo>, String> {
    let tracks = analysis::list_midi_tracks(Path::new(&midi_path))
        .map_err(|e| e.to_string())?;

    Ok(tracks
        .into_iter()
        .map(|(index, name, note_count)| MidiTrackInfo {
            index,
            name,
            note_count,
        })
        .collect())
}

/// Import MIDI file as pitch chart for a track.
#[tauri::command]
async fn import_midi_chart(
    track_id: String,
    midi_path: String,
    track_index: Option<usize>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_path = state.db_path.clone();
    let artifacts_root = state.artifacts_root.clone();

    let app_handle_clone = app_handle.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let conn = open_db(&db_path)?;
        let track = load_track(&conn, &track_id)?;

        let _ = app_handle_clone.emit(
            "analysis-progress",
            AnalysisProgress {
                phase: "midi".to_string(),
                step: 1,
                total_steps: 2,
                message: "Importing MIDI chart...".to_string(),
            },
        );

        let midi_config = analysis::MidiImportConfig {
            track_index,
            output_sample_rate: track.sample_rate,
            ..Default::default()
        };

        let pitch_contour = analysis::import_midi_to_pitch_contour(
            Path::new(&midi_path),
            track.duration_seconds,
            &midi_config,
        )
        .map_err(|e| e.to_string())?;

        // Create artifact envelope
        let run_id = uuid::Uuid::new_v4().to_string();
        let run_dir = artifacts_root.join("analysis_runs").join(&run_id);
        std::fs::create_dir_all(&run_dir)
            .map_err(|e| format!("Failed to create run dir: {e}"))?;

        let params = analysis::PitchContourParams::default();
        let created_at = chrono::Utc::now().to_rfc3339();

        let envelope = analysis::ArtifactEnvelope {
            kind: "pitch_contour".to_string(),
            schema_version: "1".to_string(),
            run_id: run_id.clone(),
            created_at: created_at.clone(),
            pipeline_version: "midi_import@0.1".to_string(),
            params_hash: analysis::sha256_bytes(&serde_json::to_vec(&params).unwrap_or_default()),
            audio_hash: track.audio_hash.clone(),
            sample_rate: track.sample_rate,
            payload: analysis::ArtifactPayload::PitchContour {
                params,
                pitch_contour,
            },
        };

        let json_path = run_dir.join("pitch_contour.json");
        let json = serde_json::to_string_pretty(&envelope)
            .map_err(|e| format!("JSON serialization failed: {e}"))?;
        std::fs::write(&json_path, json)
            .map_err(|e| format!("Failed to write artifact: {e}"))?;

        // Persist to database
        conn.execute(
            "INSERT INTO analysis_runs (id, track_id, pipeline_version, params_hash, audio_hash, status, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'success', ?6)",
            params![
                run_id,
                track_id,
                "midi_import@0.1",
                envelope.params_hash,
                track.audio_hash,
                created_at,
            ],
        )
        .map_err(|e| format!("Failed to persist analysis run: {e}"))?;

        conn.execute(
            "INSERT INTO artifacts (analysis_run_id, kind, schema_version, file_path, sample_rate, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                run_id,
                "pitch_contour",
                "1",
                json_path.to_string_lossy().to_string(),
                track.sample_rate,
                created_at,
            ],
        )
        .map_err(|e| format!("Failed to persist artifact: {e}"))?;

        let _ = app_handle_clone.emit(
            "analysis-progress",
            AnalysisProgress {
                phase: "complete".to_string(),
                step: 2,
                total_steps: 2,
                message: "MIDI chart imported successfully!".to_string(),
            },
        );

        Ok(())
    })
    .await
    .map_err(|e| format!("task join error: {e}"))?
}

// ============================================================================
// GAME ENGINE COMMANDS
// ============================================================================

/// Game session result returned when game ends.
#[derive(Debug, Clone, Serialize, Default)]
pub struct GameSessionResult {
    pub id: String,
    pub track_id: String,
    pub player_name: String,
    pub mode: GameMode,
    pub score: u64,
    pub accuracy_pct: f64,
    pub perfect_count: u32,
    pub great_count: u32,
    pub good_count: u32,
    pub ok_count: u32,
    pub miss_count: u32,
    pub max_streak: u32,
}

impl Default for GameMode {
    fn default() -> Self {
        GameMode::Solo
    }
}

/// Leaderboard entry.
#[derive(Debug, Clone, Serialize)]
pub struct LeaderboardEntry {
    pub id: String,
    pub track_id: String,
    pub player_name: String,
    pub score: f64,
    pub created_at: String,
}

/// Load target pitches from the most recent pitch contour artifact for a track.
fn load_target_pitches(conn: &Connection, track_id: &str) -> Result<Vec<TargetPitch>, String> {
    // Find the most recent pitch_contour artifact for this track
    let artifact_path: String = conn
        .query_row(
            "SELECT a.file_path FROM artifacts a
             JOIN analysis_runs r ON a.analysis_run_id = r.id
             WHERE r.track_id = ?1 AND a.kind = 'pitch_contour'
             ORDER BY a.created_at DESC LIMIT 1",
            params![track_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("No pitch contour found for track. Run analysis first: {e}"))?;

    // Read and parse the artifact
    let content = std::fs::read_to_string(&artifact_path)
        .map_err(|e| format!("Failed to read pitch artifact: {e}"))?;

    let envelope: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse pitch artifact: {e}"))?;

    let pitch_contour = envelope
        .get("pitch_contour")
        .ok_or("Missing pitch_contour in artifact")?;

    let times_s: Vec<f64> = serde_json::from_value(
        pitch_contour.get("times_s").cloned().unwrap_or_default(),
    )
    .unwrap_or_default();

    let frequencies_hz: Vec<Option<f64>> = serde_json::from_value(
        pitch_contour
            .get("frequencies_hz")
            .cloned()
            .unwrap_or_default(),
    )
    .unwrap_or_default();

    let voiced: Vec<bool> = serde_json::from_value(
        pitch_contour.get("voiced").cloned().unwrap_or_default(),
    )
    .unwrap_or_default();

    // Convert to TargetPitch structs
    let targets: Vec<TargetPitch> = times_s
        .iter()
        .enumerate()
        .map(|(i, &time_s)| TargetPitch {
            time_ms: (time_s * 1000.0) as u64,
            frequency_hz: frequencies_hz.get(i).copied().flatten(),
            voiced: *voiced.get(i).unwrap_or(&false),
        })
        .collect();

    Ok(targets)
}

/// Run the game loop in a dedicated thread.
/// This function owns all audio resources (AudioPlayer, MicCapture) to avoid Send issues.
fn run_game_thread(
    command_rx: Receiver<GameCommand>,
    result_tx: Sender<GameSessionResult>,
    app_handle: AppHandle,
) {
    let tick_interval = Duration::from_millis(20); // 50Hz
    let pitch_detector = PitchDetector::new();

    // Game state owned by this thread
    let mut player: Option<AudioPlayer> = None;
    let mut mic: Option<MicCapture> = None;
    let mut scoring = ScoringEngine::new();
    let mut target_pitches: Vec<TargetPitch> = Vec::new();
    let mut game_state = GameState::Ready;
    let mut start_time: Option<Instant> = None;
    let mut game_info: Option<GameInfo> = None;
    let mut sample_buffer: Vec<f32> = Vec::with_capacity(4096); // Accumulate samples for pitch detection

    loop {
        let loop_start = Instant::now();

        // Check for commands (non-blocking)
        while let Ok(cmd) = command_rx.try_recv() {
            match cmd {
                GameCommand::Start { audio_path, track_id, player_name, mode, target_pitches: pitches } => {
                    // Initialize audio player
                    match AudioPlayer::new(Path::new(&audio_path)) {
                        Ok(p) => player = Some(p),
                        Err(e) => {
                            eprintln!("Failed to create audio player: {e}");
                            continue;
                        }
                    }

                    // Initialize mic capture
                    match MicCapture::new() {
                        Ok(m) => {
                            println!("Microphone initialized successfully");
                            mic = Some(m);
                        }
                        Err(e) => {
                            eprintln!("Failed to initialize microphone: {e}");
                            mic = None;
                        }
                    }

                    // Reset scoring and buffer
                    scoring.reset();
                    sample_buffer.clear();
                    target_pitches = pitches.clone();
                    game_state = GameState::Ready;
                    start_time = None;
                    game_info = Some(GameInfo { track_id, player_name, mode });

                    // Send target pitches to frontend
                    #[derive(Clone, serde::Serialize)]
                    struct TargetPitchEvent {
                        time_ms: u64,
                        frequency_hz: Option<f64>,
                        voiced: bool,
                    }
                    let pitch_events: Vec<TargetPitchEvent> = pitches
                        .iter()
                        .map(|p| TargetPitchEvent {
                            time_ms: p.time_ms,
                            frequency_hz: p.frequency_hz,
                            voiced: p.voiced,
                        })
                        .collect();
                    println!("Sending {} target pitches to frontend", pitch_events.len());
                    let _ = app_handle.emit("game:target_pitches", &pitch_events);

                    let _ = app_handle.emit("game:state_change", GameStateChange {
                        state: GameState::Ready,
                        countdown: None,
                    });
                }

                GameCommand::BeginCountdown => {
                    // Run countdown
                    for i in (1..=3).rev() {
                        game_state = GameState::Countdown;
                        let _ = app_handle.emit("game:state_change", GameStateChange {
                            state: GameState::Countdown,
                            countdown: Some(i),
                        });
                        thread::sleep(Duration::from_secs(1));
                    }

                    // Start playing
                    if let Some(ref p) = player {
                        p.play();
                    }

                    // Start mic capture
                    let mic_status = if let Some(ref m) = mic {
                        m.start();
                        "Microphone active".to_string()
                    } else {
                        "No microphone available".to_string()
                    };
                    let _ = app_handle.emit("game:debug", &mic_status);

                    game_state = GameState::Playing;
                    start_time = Some(Instant::now());

                    let _ = app_handle.emit("game:state_change", GameStateChange {
                        state: GameState::Playing,
                        countdown: None,
                    });
                }

                GameCommand::Pause => {
                    if let Some(ref p) = player {
                        p.pause();
                    }
                    if let Some(ref m) = mic {
                        m.stop();
                    }
                    game_state = GameState::Paused;
                    let _ = app_handle.emit("game:state_change", GameStateChange {
                        state: GameState::Paused,
                        countdown: None,
                    });
                }

                GameCommand::Resume => {
                    if let Some(ref p) = player {
                        p.play();
                    }
                    if let Some(ref m) = mic {
                        m.start();
                    }
                    game_state = GameState::Playing;
                    let _ = app_handle.emit("game:state_change", GameStateChange {
                        state: GameState::Playing,
                        countdown: None,
                    });
                }

                GameCommand::Stop => {
                    if let Some(ref p) = player {
                        p.stop();
                    }
                    if let Some(ref m) = mic {
                        m.stop();
                    }

                    // Send result
                    let update = scoring.get_update();
                    if let Some(ref info) = game_info {
                        let result = GameSessionResult {
                            id: uuid::Uuid::new_v4().to_string(),
                            track_id: info.track_id.clone(),
                            player_name: info.player_name.clone(),
                            mode: info.mode,
                            score: update.score,
                            accuracy_pct: update.accuracy_pct,
                            perfect_count: update.perfect_count,
                            great_count: update.great_count,
                            good_count: update.good_count,
                            ok_count: update.ok_count,
                            miss_count: update.miss_count,
                            max_streak: update.max_streak,
                        };
                        let _ = result_tx.send(result);
                    }

                    game_state = GameState::Finished;
                    player = None;
                    game_info = None;

                    let _ = app_handle.emit("game:state_change", GameStateChange {
                        state: GameState::Finished,
                        countdown: None,
                    });
                }

                GameCommand::Shutdown => {
                    return;
                }
            }
        }

        // Game loop tick
        if game_state == GameState::Playing {
            if let Some(ref p) = player {
                // Emit playback tick
                let tick = p.get_tick();
                let _ = app_handle.emit("game:playback_tick", &tick);

                // Process microphone input
                if let Some(ref mut m) = mic {
                    let new_samples = m.read_samples();

                    // Accumulate samples in buffer
                    sample_buffer.extend(new_samples);

                    // Keep buffer from growing too large (keep last 4096 samples)
                    let max_buffer = 4096;
                    if sample_buffer.len() > max_buffer {
                        let drain_count = sample_buffer.len() - max_buffer;
                        sample_buffer.drain(0..drain_count);
                    }

                    let sample_count = sample_buffer.len();
                    let window_size = pitch_detector.window_size();

                    // Debug: periodically log sample count
                    static DEBUG_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
                    let count = DEBUG_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if count % 50 == 0 {
                        let _ = app_handle.emit("game:debug", format!(
                            "Mic buffer: {} (need {})",
                            sample_count,
                            window_size
                        ));
                    }

                    if sample_count >= window_size {
                        let elapsed_ms = start_time
                            .map(|t| t.elapsed().as_millis() as u64)
                            .unwrap_or(0);

                        // Use the most recent samples for detection
                        let detect_samples = &sample_buffer[sample_buffer.len() - window_size..];
                        let pitch_event = pitch_detector.detect_event(detect_samples, elapsed_ms);

                        // Debug: log when pitch is detected
                        if pitch_event.pitch_hz.is_some() {
                            let _ = app_handle.emit("game:debug", format!(
                                "Pitch: {:.1} Hz ({:.0}%)",
                                pitch_event.pitch_hz.unwrap(),
                                pitch_event.confidence * 100.0
                            ));
                        }

                        let _ = app_handle.emit("game:user_pitch", &pitch_event);

                        // Score the pitch
                        if let Some(freq) = pitch_event.pitch_hz {
                            let position_ms = tick.position_ms;
                            if let Some(target) = target_pitches.iter().find(|t| {
                                t.voiced && (t.time_ms as i64 - position_ms as i64).abs() < 50
                            }) {
                                if let Some(target_freq) = target.frequency_hz {
                                    scoring.process_hit(freq, target_freq);
                                }
                            }
                        }

                        // Emit score update
                        let update = scoring.get_update();
                        let _ = app_handle.emit("game:score_update", &update);
                    }
                }

                // Check if playback finished
                if p.state() != PlaybackState::Playing && p.position_ms() >= p.duration_ms().saturating_sub(100) {
                    game_state = GameState::Finished;

                    // Send result
                    let update = scoring.get_update();
                    if let Some(ref info) = game_info {
                        let result = GameSessionResult {
                            id: uuid::Uuid::new_v4().to_string(),
                            track_id: info.track_id.clone(),
                            player_name: info.player_name.clone(),
                            mode: info.mode,
                            score: update.score,
                            accuracy_pct: update.accuracy_pct,
                            perfect_count: update.perfect_count,
                            great_count: update.great_count,
                            good_count: update.good_count,
                            ok_count: update.ok_count,
                            miss_count: update.miss_count,
                            max_streak: update.max_streak,
                        };
                        let _ = result_tx.send(result);
                    }

                    let _ = app_handle.emit("game:state_change", GameStateChange {
                        state: GameState::Finished,
                        countdown: None,
                    });
                }
            }
        }

        // Sleep for remaining tick time
        let elapsed = loop_start.elapsed();
        if elapsed < tick_interval {
            thread::sleep(tick_interval - elapsed);
        }
    }
}

/// Start a game session.
#[tauri::command]
async fn start_game(
    track_id: String,
    player_name: String,
    mode: GameMode,
    state: State<'_, AppState>,
    game_state: State<'_, Arc<Mutex<GameControlState>>>,
) -> Result<(), String> {
    let db_path = state.db_path.clone();

    // Load track info
    let conn = open_db(&db_path)?;
    let track = load_track(&conn, &track_id)?;

    // Load target pitches
    let target_pitches = load_target_pitches(&conn, &track_id)?;

    // Send command to game thread
    let mut gs = game_state.lock();
    gs.current_game = Some(GameInfo {
        track_id: track_id.clone(),
        player_name: player_name.clone(),
        mode,
    });

    gs.command_tx
        .send(GameCommand::Start {
            audio_path: track.audio_path,
            track_id,
            player_name,
            mode,
            target_pitches,
        })
        .map_err(|e| format!("Failed to send command: {e}"))?;

    Ok(())
}

/// Begin countdown and start playing.
#[tauri::command]
fn begin_countdown(
    game_state: State<'_, Arc<Mutex<GameControlState>>>,
) -> Result<(), String> {
    let gs = game_state.lock();
    gs.command_tx
        .send(GameCommand::BeginCountdown)
        .map_err(|e| format!("Failed to send command: {e}"))?;
    Ok(())
}

/// Pause the game.
#[tauri::command]
fn pause_game(
    game_state: State<'_, Arc<Mutex<GameControlState>>>,
) -> Result<(), String> {
    let gs = game_state.lock();
    gs.command_tx
        .send(GameCommand::Pause)
        .map_err(|e| format!("Failed to send command: {e}"))?;
    Ok(())
}

/// Resume the game.
#[tauri::command]
fn resume_game(
    game_state: State<'_, Arc<Mutex<GameControlState>>>,
) -> Result<(), String> {
    let gs = game_state.lock();
    gs.command_tx
        .send(GameCommand::Resume)
        .map_err(|e| format!("Failed to send command: {e}"))?;
    Ok(())
}

/// Stop the game and get results.
#[tauri::command]
async fn stop_game(
    state: State<'_, AppState>,
    game_state: State<'_, Arc<Mutex<GameControlState>>>,
) -> Result<Option<GameSessionResult>, String> {
    // Send stop command and wait for result
    let result = {
        let gs = game_state.lock();
        let _ = gs.command_tx.send(GameCommand::Stop);

        // Try to receive result with timeout
        gs.result_rx.recv_timeout(Duration::from_millis(500)).ok()
    };

    // Save to database if we have a result
    if let Some(ref r) = result {
        let conn = open_db(&state.db_path)?;
        let created_at = chrono::Utc::now().to_rfc3339();
        let mode_str = match r.mode {
            GameMode::Solo => "solo",
            GameMode::Competition => "competition",
        };

        conn.execute(
            "INSERT INTO game_sessions (id, track_id, player_name, mode, score, accuracy_pct,
             perfect_count, great_count, good_count, ok_count, miss_count, max_streak, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                r.id,
                r.track_id,
                r.player_name,
                mode_str,
                r.score as f64,
                r.accuracy_pct,
                r.perfect_count,
                r.great_count,
                r.good_count,
                r.ok_count,
                r.miss_count,
                r.max_streak,
                created_at
            ],
        )
        .map_err(|e| format!("Failed to save game session: {e}"))?;

        // Update leaderboard
        let leaderboard_id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO leaderboards (id, track_id, player_name, score, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![leaderboard_id, r.track_id, r.player_name, r.score as f64, created_at],
        )
        .map_err(|e| format!("Failed to update leaderboard: {e}"))?;
    }

    Ok(result)
}

/// Get leaderboard for a track.
#[tauri::command]
fn get_leaderboard(
    track_id: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<LeaderboardEntry>, String> {
    let conn = open_db(&state.db_path)?;
    let limit = limit.unwrap_or(10);

    let mut stmt = conn
        .prepare(
            "SELECT id, track_id, player_name, score, created_at
             FROM leaderboards
             WHERE track_id = ?1
             ORDER BY score DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare query: {e}"))?;

    let entries = stmt
        .query_map(params![track_id, limit], |row| {
            Ok(LeaderboardEntry {
                id: row.get(0)?,
                track_id: row.get(1)?,
                player_name: row.get(2)?,
                score: row.get(3)?,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| format!("Failed to query leaderboard: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read leaderboard: {e}"))?;

    Ok(entries)
}

/// Get game sessions for a track.
#[tauri::command]
fn get_game_sessions(
    track_id: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<GameSessionResult>, String> {
    let conn = open_db(&state.db_path)?;
    let limit = limit.unwrap_or(20);

    let mut stmt = conn
        .prepare(
            "SELECT id, track_id, player_name, mode, score, accuracy_pct,
             perfect_count, great_count, good_count, ok_count, miss_count, max_streak
             FROM game_sessions
             WHERE track_id = ?1
             ORDER BY created_at DESC
             LIMIT ?2",
        )
        .map_err(|e| format!("Failed to prepare query: {e}"))?;

    let sessions = stmt
        .query_map(params![track_id, limit], |row| {
            let mode_str: String = row.get(3)?;
            let mode = if mode_str == "competition" {
                GameMode::Competition
            } else {
                GameMode::Solo
            };

            Ok(GameSessionResult {
                id: row.get(0)?,
                track_id: row.get(1)?,
                player_name: row.get(2)?,
                mode,
                score: row.get::<_, f64>(4)? as u64,
                accuracy_pct: row.get(5)?,
                perfect_count: row.get(6)?,
                great_count: row.get(7)?,
                good_count: row.get(8)?,
                ok_count: row.get(9)?,
                miss_count: row.get(10)?,
                max_streak: row.get(11)?,
            })
        })
        .map_err(|e| format!("Failed to query sessions: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to read sessions: {e}"))?;

    Ok(sessions)
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

            // Create channels for game thread communication
            let (command_tx, command_rx) = bounded::<GameCommand>(16);
            let (result_tx, result_rx) = bounded::<GameSessionResult>(4);

            // Clone app_handle for the game thread
            let game_app_handle = app_handle.clone();

            // Start game thread
            thread::spawn(move || {
                run_game_thread(command_rx, result_tx, game_app_handle);
            });

            // Initialize game control state
            app.manage(Arc::new(Mutex::new(GameControlState {
                command_tx,
                result_rx,
                current_game: None,
            })));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Track management
            import_track,
            list_tracks,
            // Analysis
            analyze_audio_file,
            analyze_pitch_contour,
            analyze_track,
            // Vocal isolation & MIDI import
            check_demucs_available,
            debug_demucs_environment,
            analyze_track_with_vocals,
            list_midi_tracks,
            import_midi_chart,
            // Game engine
            start_game,
            begin_countdown,
            pause_game,
            resume_game,
            stop_game,
            get_leaderboard,
            get_game_sessions
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
