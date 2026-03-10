//! Microphone capture using cpal with a lock-free ring buffer.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, InputCallbackInfo, SampleFormat, Stream, StreamConfig};
use ringbuf::{HeapConsumer, HeapProducer, HeapRb};
use serde::{Deserialize, Serialize};

/// Configuration for microphone capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicConfig {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Ring buffer size in samples (should be power of 2).
    pub buffer_size: usize,
}

impl Default for MicConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            buffer_size: 4096, // ~93ms at 44.1kHz
        }
    }
}

/// Microphone capture with ring buffer for pitch detection.
pub struct MicCapture {
    /// The audio input stream (must be kept alive).
    _stream: Stream,
    /// Consumer end of the ring buffer for reading samples.
    consumer: HeapConsumer<f32>,
    /// Whether capture is active.
    is_capturing: Arc<AtomicBool>,
    /// Sample rate.
    sample_rate: u32,
}

impl MicCapture {
    /// Create a new MicCapture with default configuration.
    pub fn new() -> Result<Self> {
        Self::with_config(MicConfig::default())
    }

    /// Create a new MicCapture with custom configuration.
    pub fn with_config(config: MicConfig) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("no default input device available")?;

        let supported_config = device
            .default_input_config()
            .context("failed to get default input config")?;

        let sample_rate = supported_config.sample_rate().0;
        let channels = supported_config.channels() as usize;

        let stream_config = StreamConfig {
            channels: supported_config.channels(),
            sample_rate: supported_config.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        // Create ring buffer
        let ring = HeapRb::<f32>::new(config.buffer_size);
        let (producer, consumer) = ring.split();

        let is_capturing = Arc::new(AtomicBool::new(false));
        let is_capturing_clone = Arc::clone(&is_capturing);

        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => Self::build_stream::<f32>(
                &device,
                &stream_config,
                producer,
                is_capturing_clone,
                channels,
            )?,
            SampleFormat::I16 => Self::build_stream::<i16>(
                &device,
                &stream_config,
                producer,
                is_capturing_clone,
                channels,
            )?,
            SampleFormat::U16 => Self::build_stream::<u16>(
                &device,
                &stream_config,
                producer,
                is_capturing_clone,
                channels,
            )?,
            _ => anyhow::bail!("unsupported sample format"),
        };

        Ok(Self {
            _stream: stream,
            consumer,
            is_capturing,
            sample_rate,
        })
    }

    fn build_stream<T: cpal::Sample + cpal::SizedSample>(
        device: &cpal::Device,
        config: &StreamConfig,
        mut producer: HeapProducer<f32>,
        is_capturing: Arc<AtomicBool>,
        channels: usize,
    ) -> Result<Stream>
    where
        f32: FromSample<T>,
    {
        let stream = device.build_input_stream(
            config,
            move |data: &[T], _: &InputCallbackInfo| {
                if !is_capturing.load(Ordering::Relaxed) {
                    return;
                }

                // Mix down to mono and push to ring buffer
                for chunk in data.chunks(channels) {
                    let mono_sample: f32 = chunk
                        .iter()
                        .map(|s| f32::from_sample_(*s))
                        .sum::<f32>()
                        / channels as f32;

                    // Best-effort push (drops if full)
                    let _ = producer.push(mono_sample);
                }
            },
            |err| {
                eprintln!("mic capture error: {}", err);
            },
            None,
        )?;

        stream.play()?;
        Ok(stream)
    }

    /// Start capturing audio.
    pub fn start(&self) {
        self.is_capturing.store(true, Ordering::Relaxed);
    }

    /// Stop capturing audio.
    pub fn stop(&self) {
        self.is_capturing.store(false, Ordering::Relaxed);
    }

    /// Check if currently capturing.
    pub fn is_capturing(&self) -> bool {
        self.is_capturing.load(Ordering::Relaxed)
    }

    /// Read available samples from the ring buffer.
    /// Returns a vector of mono samples.
    pub fn read_samples(&mut self) -> Vec<f32> {
        let available = self.consumer.len();
        let mut samples = vec![0.0f32; available];
        self.consumer.pop_slice(&mut samples);
        samples
    }

    /// Read exactly `count` samples if available.
    /// Returns None if not enough samples are available.
    pub fn read_exact(&mut self, count: usize) -> Option<Vec<f32>> {
        if self.consumer.len() >= count {
            let mut samples = vec![0.0f32; count];
            self.consumer.pop_slice(&mut samples);
            Some(samples)
        } else {
            None
        }
    }

    /// Get the number of samples available to read.
    pub fn available(&self) -> usize {
        self.consumer.len()
    }

    /// Get the sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Clear all buffered samples.
    pub fn clear(&mut self) {
        while self.consumer.pop().is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mic_config_default() {
        let config = MicConfig::default();
        assert_eq!(config.sample_rate, 44100);
        assert_eq!(config.buffer_size, 4096);
    }
}
