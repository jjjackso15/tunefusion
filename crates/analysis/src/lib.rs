use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use uuid::Uuid;

use audio_engine::{decode_to_pcm, mixdown_mono};

pub const ARTIFACT_KIND_WAVEFORM_PEAKS: &str = "waveform_peaks";
pub const SCHEMA_VERSION: &str = "1";
pub const PIPELINE_VERSION: &str = "waveform_peaks@0.1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformParams {
    /// Number of peak buckets across the track.
    pub buckets: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactEnvelope {
    pub kind: String,
    pub schema_version: String,

    pub run_id: String,
    pub created_at: String,

    pub pipeline_version: String,
    pub params: WaveformParams,
    pub params_hash: String,

    pub audio_hash: String,
    pub sample_rate: u32,

    /// Peak magnitude per bucket (0..1, best-effort)
    pub waveform_peaks: Vec<f32>,
}

/// Analyze an audio file, compute waveform peaks, and write an artifact JSON file.
///
/// Artifact-first rules:
/// - Never overwrites: writes under `artifacts_root/analysis_runs/<run_id>/waveform_peaks.json`.
/// - Includes `pipeline_version`, `params`, and `params_hash`.
/// - Includes `audio_hash` (sha256 of source audio bytes).
pub fn analyze_waveform_to_artifact_json(
    audio_path: &Path,
    artifacts_root: &Path,
) -> Result<ArtifactEnvelope> {
    let params = WaveformParams { buckets: 256 };

    let audio_hash = sha256_file(audio_path)
        .with_context(|| format!("failed hashing audio file: {}", audio_path.display()))?;

    let params_hash = sha256_bytes(&serde_json::to_vec(&params)?);

    let pcm = decode_to_pcm(audio_path)
        .with_context(|| format!("decode failed: {}", audio_path.display()))?;
    let samples_mono = mixdown_mono(&pcm);

    let waveform_peaks = compute_waveform_peaks(&samples_mono, params.buckets);

    let run_id = Uuid::new_v4().to_string();
    let run_dir = artifacts_root.join("analysis_runs").join(&run_id);
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("failed to create run dir: {}", run_dir.display()))?;

    let envelope = ArtifactEnvelope {
        kind: ARTIFACT_KIND_WAVEFORM_PEAKS.to_string(),
        schema_version: SCHEMA_VERSION.to_string(),
        run_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        pipeline_version: PIPELINE_VERSION.to_string(),
        params,
        params_hash,
        audio_hash,
        sample_rate: pcm.sample_rate,
        waveform_peaks,
    };

    let json_path = run_dir.join("waveform_peaks.json");
    let json = serde_json::to_string_pretty(&envelope)?;
    fs::write(&json_path, json)
        .with_context(|| format!("failed writing artifact json: {}", json_path.display()))?;

    Ok(envelope)
}

/// Compute peak magnitude per bucket across samples.
pub fn compute_waveform_peaks(samples: &[f32], buckets: usize) -> Vec<f32> {
    if samples.is_empty() || buckets == 0 {
        return Vec::new();
    }

    let chunk_size = (samples.len() as f64 / buckets as f64).ceil() as usize;
    samples
        .chunks(chunk_size.max(1))
        .map(|chunk| chunk.iter().fold(0.0_f32, |acc, &s| acc.max(s.abs())))
        .collect()
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    Ok(sha256_bytes(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn computes_expected_peaks() {
        let samples = vec![-0.1, 0.25, -0.8, 0.3, -0.5, 0.1, -0.2, 0.9];
        let peaks = compute_waveform_peaks(&samples, 4);
        assert_eq!(peaks, vec![0.25, 0.8, 0.5, 0.9]);
    }

    #[test]
    fn writes_waveform_artifact_json() {
        // generate a small wav file
        let dir = tempdir().unwrap();
        let wav_path = dir.path().join("test.wav");
        let mut writer = hound::WavWriter::create(
            &wav_path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 44_100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();

        for sample in [0_i16, 1000, -1000, 2000, -2000, 3000, -3000] {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();

        let artifact = analyze_waveform_to_artifact_json(&wav_path, dir.path()).unwrap();

        let artifact_path = dir
            .path()
            .join("analysis_runs")
            .join(&artifact.run_id)
            .join("waveform_peaks.json");

        assert!(artifact_path.exists());

        // schema validation: can read back into the same struct
        let content = fs::read_to_string(artifact_path).unwrap();
        let parsed: ArtifactEnvelope = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.kind, ARTIFACT_KIND_WAVEFORM_PEAKS);
        assert_eq!(parsed.schema_version, SCHEMA_VERSION);
        assert_eq!(parsed.pipeline_version, PIPELINE_VERSION);
        assert_eq!(parsed.params_hash.len(), 64);
        assert_eq!(parsed.audio_hash.len(), 64);
        assert!(!parsed.waveform_peaks.is_empty());
    }
}
