use anyhow::{Context, Result};
use pyin::{Framing, PYINExecutor, PadMode};
use serde::{Deserialize, Serialize};
use std::path::Path;

use audio_engine::{decode_to_pcm, mixdown_mono};

use crate::{
    create_run_dir, sha256_bytes, sha256_file, write_artifact, ArtifactEnvelope, ArtifactPayload,
};

pub const ARTIFACT_KIND_PITCH_CONTOUR: &str = "pitch_contour";
pub const PITCH_CONTOUR_SCHEMA_VERSION: &str = "1";
pub const PITCH_CONTOUR_PIPELINE_VERSION: &str = "pitch_contour@0.1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchContourParams {
    /// Minimum expected frequency in Hz (e.g. 65.0 for C2).
    pub fmin_hz: f64,
    /// Maximum expected frequency in Hz (e.g. 1047.0 for C6).
    pub fmax_hz: f64,
    /// FFT frame length in samples.
    pub frame_length: usize,
    /// Hop length in samples.
    pub hop_length: usize,
    /// Frequency resolution in semitone fractions (0.1 = 10 bins per semitone).
    pub resolution: f64,
}

impl Default for PitchContourParams {
    fn default() -> Self {
        Self {
            // Vocal range focused: skip bass guitar (~40-120 Hz)
            // Male vocals: ~85-350 Hz fundamental
            // Female vocals: ~165-500 Hz fundamental
            fmin_hz: 150.0,  // Start above bass guitar to focus on vocals
            fmax_hz: 800.0,  // Upper vocal fundamental range
            frame_length: 2048,
            hop_length: 512,
            resolution: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchContourData {
    /// Timestamp in seconds for each frame.
    pub times_s: Vec<f64>,
    /// F0 estimate in Hz per frame (None = unvoiced).
    pub frequencies_hz: Vec<Option<f64>>,
    /// Whether each frame is voiced.
    pub voiced: Vec<bool>,
    /// Voicing probability [0.0, 1.0] per frame.
    pub voiced_prob: Vec<f64>,
}

/// Run pYIN pitch detection on mono audio samples.
pub fn compute_pitch_contour(
    samples_mono: &[f32],
    sample_rate: u32,
    params: &PitchContourParams,
) -> Result<PitchContourData> {
    let samples_f64: Vec<f64> = samples_mono.iter().map(|&s| s as f64).collect();

    let mut executor = PYINExecutor::new(
        params.fmin_hz,
        params.fmax_hz,
        sample_rate,
        params.frame_length,
        None,
        Some(params.hop_length),
        Some(params.resolution),
    );

    let (timestamps, f0, voiced_flags, voiced_probs) = executor.pyin(
        &samples_f64,
        f64::NAN,
        Framing::Center(PadMode::Constant(0.0)),
    );

    Ok(PitchContourData {
        times_s: timestamps.to_vec(),
        frequencies_hz: f0
            .iter()
            .map(|&v| if v.is_nan() { None } else { Some(v) })
            .collect(),
        voiced: voiced_flags.to_vec(),
        voiced_prob: voiced_probs.to_vec(),
    })
}

/// Analyze an audio file, compute pitch contour, and write an artifact JSON file.
pub fn analyze_pitch_contour_to_artifact_json(
    audio_path: &Path,
    artifacts_root: &Path,
) -> Result<ArtifactEnvelope> {
    let params = PitchContourParams::default();

    let audio_hash = sha256_file(audio_path)
        .with_context(|| format!("failed hashing audio file: {}", audio_path.display()))?;
    let params_hash = sha256_bytes(&serde_json::to_vec(&params)?);

    let pcm = decode_to_pcm(audio_path)
        .with_context(|| format!("decode failed: {}", audio_path.display()))?;
    let samples_mono = mixdown_mono(&pcm);

    let pitch_contour = compute_pitch_contour(&samples_mono, pcm.sample_rate, &params)?;

    let (run_id, run_dir) = create_run_dir(artifacts_root)?;

    let envelope = ArtifactEnvelope {
        kind: ARTIFACT_KIND_PITCH_CONTOUR.to_string(),
        schema_version: PITCH_CONTOUR_SCHEMA_VERSION.to_string(),
        run_id,
        created_at: chrono::Utc::now().to_rfc3339(),
        pipeline_version: PITCH_CONTOUR_PIPELINE_VERSION.to_string(),
        params_hash,
        audio_hash,
        sample_rate: pcm.sample_rate,
        payload: ArtifactPayload::PitchContour {
            params,
            pitch_contour,
        },
    };

    write_artifact(&run_dir, "pitch_contour.json", &envelope)?;
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;
    use tempfile::tempdir;

    fn write_sine_wav(path: &Path, freq_hz: f32, sample_rate: u32, seconds: f32) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        let n_samples = (sample_rate as f32 * seconds) as usize;
        for i in 0..n_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * PI * freq_hz * t).sin();
            writer
                .write_sample((sample * i16::MAX as f32) as i16)
                .unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn detects_known_pitch() {
        let dir = tempdir().unwrap();
        let wav_path = dir.path().join("a440.wav");
        write_sine_wav(&wav_path, 440.0, 44100, 1.0);

        let envelope =
            analyze_pitch_contour_to_artifact_json(&wav_path, dir.path()).unwrap();

        assert_eq!(envelope.kind, ARTIFACT_KIND_PITCH_CONTOUR);

        if let ArtifactPayload::PitchContour { pitch_contour, .. } = &envelope.payload {
            let voiced_count = pitch_contour.voiced.iter().filter(|&&v| v).count();
            assert!(voiced_count > 0, "expected some voiced frames");

            let voiced_freqs: Vec<f64> = pitch_contour
                .frequencies_hz
                .iter()
                .zip(&pitch_contour.voiced)
                .filter(|(_, &v)| v)
                .filter_map(|(f, _)| *f)
                .collect();

            assert!(!voiced_freqs.is_empty(), "expected voiced frequencies");
            let mean_freq = voiced_freqs.iter().sum::<f64>() / voiced_freqs.len() as f64;
            assert!(
                (mean_freq - 440.0).abs() < 10.0,
                "expected ~440 Hz, got {}",
                mean_freq
            );
        } else {
            panic!("wrong payload type");
        }
    }

    #[test]
    fn serialization_roundtrip() {
        let dir = tempdir().unwrap();
        let wav_path = dir.path().join("tone.wav");
        write_sine_wav(&wav_path, 440.0, 44100, 0.5);

        let envelope =
            analyze_pitch_contour_to_artifact_json(&wav_path, dir.path()).unwrap();

        let json_path = dir
            .path()
            .join("analysis_runs")
            .join(&envelope.run_id)
            .join("pitch_contour.json");

        assert!(json_path.exists());

        let content = std::fs::read_to_string(json_path).unwrap();
        let parsed: ArtifactEnvelope = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.kind, ARTIFACT_KIND_PITCH_CONTOUR);
        assert_eq!(parsed.schema_version, PITCH_CONTOUR_SCHEMA_VERSION);
        assert_eq!(parsed.pipeline_version, PITCH_CONTOUR_PIPELINE_VERSION);
    }
}
