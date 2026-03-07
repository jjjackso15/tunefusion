use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

mod pitch_contour;
mod waveform;

pub use pitch_contour::*;
pub use waveform::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactEnvelope {
    pub kind: String,
    pub schema_version: String,

    pub run_id: String,
    pub created_at: String,

    pub pipeline_version: String,
    pub params_hash: String,

    pub audio_hash: String,
    pub sample_rate: u32,

    #[serde(flatten)]
    pub payload: ArtifactPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArtifactPayload {
    WaveformPeaks {
        params: WaveformParams,
        waveform_peaks: Vec<f32>,
    },
    PitchContour {
        params: PitchContourParams,
        pitch_contour: PitchContourData,
    },
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(sha256_bytes(&bytes))
}

/// Create the run directory and return (run_id, run_dir_path).
pub fn create_run_dir(artifacts_root: &Path) -> Result<(String, PathBuf)> {
    let run_id = Uuid::new_v4().to_string();
    let run_dir = artifacts_root.join("analysis_runs").join(&run_id);
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("failed to create run dir: {}", run_dir.display()))?;
    Ok((run_id, run_dir))
}

/// Write an artifact envelope to disk as pretty-printed JSON.
pub fn write_artifact(run_dir: &Path, filename: &str, envelope: &ArtifactEnvelope) -> Result<()> {
    let json_path = run_dir.join(filename);
    let json = serde_json::to_string_pretty(envelope)?;
    fs::write(&json_path, json)
        .with_context(|| format!("failed writing artifact json: {}", json_path.display()))?;
    Ok(())
}
