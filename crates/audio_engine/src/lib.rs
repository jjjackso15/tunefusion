use anyhow::{Context, Result};
use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::default::{get_codecs, get_probe};

/// Interleaved PCM samples as f32 in [-1.0, 1.0] (best-effort).
#[derive(Debug, Clone)]
pub struct PcmBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    /// Interleaved frames: len == frame_count * channels
    pub frames: Vec<f32>,
}

/// Decode an audio file into interleaved f32 PCM.
///
/// Notes:
/// - Pure Rust decoding via Symphonia (cross-platform; Tauri-friendly).
/// - For MVP we decode the first supported audio track.
pub fn decode_to_pcm(path: &Path) -> Result<PcmBuffer> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open audio file: {}", path.display()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .with_context(|| format!("failed to probe audio format: {}", path.display()))?;

    let mut format = probed.format;

    let track_id = format
        .default_track()
        .context("no supported default audio track found")?
        .id;

    let codec_params = format
        .tracks()
        .iter()
        .find(|t| t.id == track_id)
        .context("track disappeared")?
        .codec_params
        .clone();

    let mut decoder = get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .context("failed to create decoder")?;

    let mut out_sample_rate: Option<u32> = None;
    let mut out_channels: Option<u16> = None;
    let mut frames: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(e) => return Err(e).context("failed reading next audio packet"),
        };

        // Only decode packets from the selected track.
        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphoniaError::DecodeError(_)) => continue, // skip corrupt frames
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(e) => return Err(e).context("decode error"),
        };

        let spec = *decoded.spec();
        out_sample_rate.get_or_insert(spec.rate);
        out_channels.get_or_insert(spec.channels.count() as u16);

        // Convert whatever sample format into interleaved f32.
        let mut sb = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
        sb.copy_interleaved_ref(decoded);
        frames.extend_from_slice(sb.samples());
    }

    Ok(PcmBuffer {
        sample_rate: out_sample_rate.unwrap_or(44_100),
        channels: out_channels.unwrap_or(1),
        frames,
    })
}

/// Mix down interleaved audio to mono by averaging channels per frame.
pub fn mixdown_mono(pcm: &PcmBuffer) -> Vec<f32> {
    let ch = pcm.channels.max(1) as usize;
    if ch == 1 {
        return pcm.frames.clone();
    }

    pcm.frames
        .chunks(ch)
        .map(|frame| frame.iter().copied().sum::<f32>() / ch as f32)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::f32::consts::PI;
    use tempfile::tempdir;

    fn write_test_wav_mono_16bit(
        path: &Path,
        sample_rate: u32,
        seconds: f32,
    ) -> anyhow::Result<()> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec)?;
        let total_samples = (sample_rate as f32 * seconds).round() as usize;
        let freq = 440.0_f32;

        for n in 0..total_samples {
            let t = n as f32 / sample_rate as f32;
            let x = (2.0 * PI * freq * t).sin();
            let s_i16 = (x * i16::MAX as f32) as i16;
            writer.write_sample(s_i16)?;
        }

        writer.finalize()?;
        Ok(())
    }

    #[test]
    fn decode_wav_mono_returns_expected_shape() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let wav_path = dir.path().join("tone.wav");

        let sr = 44_100u32;
        let seconds = 0.25_f32;
        write_test_wav_mono_16bit(&wav_path, sr, seconds)?;

        let pcm = decode_to_pcm(&wav_path)?;

        assert_eq!(pcm.sample_rate, sr);
        assert_eq!(pcm.channels, 1);

        let expected_samples = (sr as f32 * seconds).round() as usize;
        assert!(
            (pcm.frames.len() as isize - expected_samples as isize).abs() <= 2,
            "frames.len()={} expected~={}",
            pcm.frames.len(),
            expected_samples
        );

        let max_abs = pcm.frames.iter().copied().map(f32::abs).fold(0.0, f32::max);
        assert!(max_abs <= 1.05, "max_abs={}", max_abs);

        assert_relative_eq!(pcm.frames[0], 0.0, epsilon = 0.05);
        Ok(())
    }

    #[test]
    fn mixdown_mono_is_identity_for_mono() {
        let pcm = PcmBuffer {
            sample_rate: 48_000,
            channels: 1,
            frames: vec![0.1, -0.2, 0.3],
        };
        let mono = mixdown_mono(&pcm);
        assert_eq!(mono, pcm.frames);
    }
}
