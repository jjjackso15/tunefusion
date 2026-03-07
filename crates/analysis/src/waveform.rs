use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use audio_engine::{decode_to_pcm, mixdown_mono};

use crate::{
    create_run_dir, sha256_bytes, sha256_file, write_artifact, ArtifactEnvelope, ArtifactPayload,
};

pub const ARTIFACT_KIND_WAVEFORM_PEAKS: &str = "waveform_peaks";
pub const WAVEFORM_SCHEMA_VERSION: &str = "1";
pub const WAVEFORM_PIPELINE_VERSION: &str = "waveform_peaks@0.1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformParams {
    /// Number of peak buckets across the track.
    pub buckets: usize,
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

/// Analyze an audio file, compute waveform peaks, and write an artifact JSON file.
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

    let (run_id, run_dir) = create_run_dir(artifacts_root)?;

    let envelope = ArtifactEnvelope {
        kind: ARTIFACT_KIND_WAVEFORM_PEAKS.to_string(),
        schema_version: WAVEFORM_SCHEMA_VERSION.to_string(),
        run_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        pipeline_version: WAVEFORM_PIPELINE_VERSION.to_string(),
        params_hash,
        audio_hash,
        sample_rate: pcm.sample_rate,
        payload: ArtifactPayload::WaveformPeaks {
            params,
            waveform_peaks,
        },
    };

    write_artifact(&run_dir, "waveform_peaks.json", &envelope)?;

    Ok(envelope)
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

        let content = std::fs::read_to_string(artifact_path).unwrap();
        let parsed: ArtifactEnvelope = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.kind, ARTIFACT_KIND_WAVEFORM_PEAKS);
        assert_eq!(parsed.schema_version, WAVEFORM_SCHEMA_VERSION);
        assert_eq!(parsed.pipeline_version, WAVEFORM_PIPELINE_VERSION);
        assert_eq!(parsed.params_hash.len(), 64);
        assert_eq!(parsed.audio_hash.len(), 64);
        if let ArtifactPayload::WaveformPeaks { waveform_peaks, .. } = &parsed.payload {
            assert!(!waveform_peaks.is_empty());
        } else {
            panic!("wrong payload type");
        }
    }
}
