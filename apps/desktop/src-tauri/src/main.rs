#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

#[tauri::command]
fn analyze_audio_file(audio_path: String) -> Result<analysis::ArtifactEnvelope, String> {
    let artifacts_root = PathBuf::from("artifacts");

    analysis::analyze_waveform_to_artifact_json(Path::new(&audio_path), &artifacts_root)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn analyze_pitch_contour(audio_path: String) -> Result<analysis::ArtifactEnvelope, String> {
    let artifacts_root = PathBuf::from("artifacts");

    analysis::analyze_pitch_contour_to_artifact_json(Path::new(&audio_path), &artifacts_root)
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            analyze_audio_file,
            analyze_pitch_contour
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
