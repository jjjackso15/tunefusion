#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

#[tauri::command]
fn analyze_audio_file(audio_path: String) -> Result<analysis::AnalysisArtifact, String> {
    let artifacts_root = PathBuf::from("artifacts");
    analysis::analyze_audio_to_artifact(PathBuf::from(audio_path).as_path(), &artifacts_root)
        .map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![analyze_audio_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
