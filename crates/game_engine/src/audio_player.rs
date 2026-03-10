//! Audio playback with sample-accurate position tracking using cpal.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{OutputCallbackInfo, SampleFormat, Stream, StreamConfig};
use serde::{Deserialize, Serialize};

use audio_engine::decode_to_pcm;

/// Playback state enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

/// Event emitted at regular intervals during playback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackTick {
    /// Current position in milliseconds.
    pub position_ms: u64,
    /// Whether audio is currently playing.
    pub is_playing: bool,
    /// Total duration in milliseconds.
    pub duration_ms: u64,
}

/// Shared state between audio callback and main thread.
struct SharedState {
    /// Current sample position (atomic for lock-free access).
    position_samples: AtomicU64,
    /// Whether playback is active.
    is_playing: AtomicBool,
    /// Total samples in the buffer.
    total_samples: u64,
    /// Sample rate.
    sample_rate: u32,
    /// Number of channels.
    channels: u16,
}

/// Audio player with real-time position tracking.
pub struct AudioPlayer {
    /// The audio output stream (must be kept alive).
    _stream: Stream,
    /// Shared state for position tracking.
    state: Arc<SharedState>,
    /// Audio samples (owned for lifetime).
    _samples: Arc<Vec<f32>>,
}

impl AudioPlayer {
    /// Create a new AudioPlayer and load audio from the given path.
    pub fn new(audio_path: &Path) -> Result<Self> {
        // Decode audio file
        let pcm = decode_to_pcm(audio_path)
            .with_context(|| format!("failed to decode audio: {}", audio_path.display()))?;

        let samples = Arc::new(pcm.frames);
        let sample_rate = pcm.sample_rate;
        let channels = pcm.channels;
        let total_samples = samples.len() as u64 / channels as u64;

        // Create shared state
        let state = Arc::new(SharedState {
            position_samples: AtomicU64::new(0),
            is_playing: AtomicBool::new(false),
            total_samples,
            sample_rate,
            channels,
        });

        // Set up cpal output
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no default output device available")?;

        let supported_config = device
            .default_output_config()
            .context("failed to get default output config")?;

        let config = StreamConfig {
            channels: channels,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        // Clone for the audio callback
        let samples_clone = Arc::clone(&samples);
        let state_clone = Arc::clone(&state);
        let ch = channels as usize;

        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => Self::build_stream::<f32>(
                &device,
                &config,
                samples_clone,
                state_clone,
                ch,
            )?,
            SampleFormat::I16 => Self::build_stream::<i16>(
                &device,
                &config,
                samples_clone,
                state_clone,
                ch,
            )?,
            SampleFormat::U16 => Self::build_stream::<u16>(
                &device,
                &config,
                samples_clone,
                state_clone,
                ch,
            )?,
            _ => anyhow::bail!("unsupported sample format"),
        };

        Ok(Self {
            _stream: stream,
            state,
            _samples: samples,
        })
    }

    fn build_stream<T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>>(
        device: &cpal::Device,
        config: &StreamConfig,
        samples: Arc<Vec<f32>>,
        state: Arc<SharedState>,
        channels: usize,
    ) -> Result<Stream> {
        let stream = device.build_output_stream(
            config,
            move |data: &mut [T], _: &OutputCallbackInfo| {
                let is_playing = state.is_playing.load(Ordering::Relaxed);
                if !is_playing {
                    // Output silence when paused
                    for sample in data.iter_mut() {
                        *sample = T::from_sample(0.0f32);
                    }
                    return;
                }

                let mut pos = state.position_samples.load(Ordering::Relaxed) as usize;
                let total = samples.len() / channels;

                for frame in data.chunks_mut(channels) {
                    if pos >= total {
                        // End of audio - output silence and stop
                        for sample in frame.iter_mut() {
                            *sample = T::from_sample(0.0f32);
                        }
                        state.is_playing.store(false, Ordering::Relaxed);
                    } else {
                        // Output audio
                        let base_idx = pos * channels;
                        for (i, sample) in frame.iter_mut().enumerate() {
                            let idx = base_idx + i;
                            let value = if idx < samples.len() {
                                samples[idx]
                            } else {
                                0.0
                            };
                            *sample = T::from_sample(value);
                        }
                        pos += 1;
                    }
                }

                state.position_samples.store(pos as u64, Ordering::Relaxed);
            },
            |err| {
                eprintln!("audio stream error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        Ok(stream)
    }

    /// Start or resume playback.
    pub fn play(&self) {
        self.state.is_playing.store(true, Ordering::Relaxed);
    }

    /// Pause playback.
    pub fn pause(&self) {
        self.state.is_playing.store(false, Ordering::Relaxed);
    }

    /// Stop playback and reset to beginning.
    pub fn stop(&self) {
        self.state.is_playing.store(false, Ordering::Relaxed);
        self.state.position_samples.store(0, Ordering::Relaxed);
    }

    /// Seek to a position in milliseconds.
    pub fn seek(&self, position_ms: u64) {
        let sample_pos = (position_ms as f64 / 1000.0 * self.state.sample_rate as f64) as u64;
        let clamped = sample_pos.min(self.state.total_samples);
        self.state.position_samples.store(clamped, Ordering::Relaxed);
    }

    /// Get current playback state.
    pub fn state(&self) -> PlaybackState {
        if self.state.is_playing.load(Ordering::Relaxed) {
            PlaybackState::Playing
        } else if self.state.position_samples.load(Ordering::Relaxed) == 0 {
            PlaybackState::Stopped
        } else {
            PlaybackState::Paused
        }
    }

    /// Get current position in milliseconds.
    pub fn position_ms(&self) -> u64 {
        let samples = self.state.position_samples.load(Ordering::Relaxed);
        (samples as f64 * 1000.0 / self.state.sample_rate as f64) as u64
    }

    /// Get total duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        (self.state.total_samples as f64 * 1000.0 / self.state.sample_rate as f64) as u64
    }

    /// Get a playback tick for the current state.
    pub fn get_tick(&self) -> PlaybackTick {
        PlaybackTick {
            position_ms: self.position_ms(),
            is_playing: self.state.is_playing.load(Ordering::Relaxed),
            duration_ms: self.duration_ms(),
        }
    }

    /// Get sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.state.sample_rate
    }
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
    fn test_audio_player_creation() {
        let dir = tempdir().unwrap();
        let wav_path = dir.path().join("test.wav");
        write_sine_wav(&wav_path, 440.0, 44100, 1.0);

        // Note: This test may fail in CI without audio device
        // In a real test environment, we'd mock the audio device
        let result = AudioPlayer::new(&wav_path);
        // Just check it doesn't panic on construction
        if result.is_ok() {
            let player = result.unwrap();
            assert_eq!(player.state(), PlaybackState::Stopped);
            assert!(player.duration_ms() > 900 && player.duration_ms() < 1100);
        }
    }
}
