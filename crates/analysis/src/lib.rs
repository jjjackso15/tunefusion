use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const PIPELINE_VERSION: &str = "v0.waveform.1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisArtifact {
    pub run_id: String,
    pub pipeline_version: String,
    pub source_audio_path: String,
    pub sample_rate: u32,
    pub waveform_peaks: Vec<f32>,
}

pub fn analyze_audio_to_artifact(audio_path: &Path, artifacts_root: &Path) -> Result<AnalysisArtifact> {
    let (sample_rate, samples) = load_wav_mono_samples(audio_path)?;
    let waveform_peaks = compute_waveform_peaks(&samples, 256);

    let run_id = Uuid::new_v4().to_string();
    let run_dir = artifacts_root.join("analysis_runs").join(&run_id);
    fs::create_dir_all(&run_dir)
        .with_context(|| format!("failed to create artifact run directory: {}", run_dir.display()))?;

    let artifact = AnalysisArtifact {
        run_id,
        pipeline_version: PIPELINE_VERSION.to_string(),
        source_audio_path: audio_path.display().to_string(),
        sample_rate,
        waveform_peaks,
    };

    let analysis_json_path = run_dir.join("analysis.json");
    let json = serde_json::to_string_pretty(&artifact)?;
    fs::write(&analysis_json_path, json)
        .with_context(|| format!("failed writing artifact: {}", analysis_json_path.display()))?;

    Ok(artifact)
}

pub fn load_wav_mono_samples(path: &Path) -> Result<(u32, Vec<f32>)> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("failed to open wav file: {}", path.display()))?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels.max(1) as usize;

    let all_samples: Vec<f32> = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Int, 16) => reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
            .collect::<std::result::Result<Vec<_>, _>>()?,
        (hound::SampleFormat::Int, 24 | 32) => reader
            .samples::<i32>()
            .map(|s| s.map(|v| v as f32 / i32::MAX as f32))
            .collect::<std::result::Result<Vec<_>, _>>()?,
        (hound::SampleFormat::Float, 32) => reader
            .samples::<f32>()
            .collect::<std::result::Result<Vec<_>, _>>()?,
        _ => anyhow::bail!(
            "unsupported wav format: {:?} {} bits",
            spec.sample_format,
            spec.bits_per_sample
        ),
    };

    if channels == 1 {
        return Ok((sample_rate, all_samples));
    }

    let mono = all_samples
        .chunks(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
        .collect::<Vec<_>>();

    Ok((sample_rate, mono))
}

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
    fn writes_analysis_artifact_json() {
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

        let artifact = analyze_audio_to_artifact(&wav_path, dir.path()).unwrap();
        let artifact_path = dir
            .path()
            .join("analysis_runs")
            .join(&artifact.run_id)
            .join("analysis.json");

        assert!(artifact_path.exists());
        let content = fs::read_to_string(artifact_path).unwrap();
        assert!(content.contains("\"waveform_peaks\""));
        assert!(content.contains("\"pipeline_version\""));
    }
}
