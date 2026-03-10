//! Real-time pitch detection using the McLeod (MPM) algorithm.

use pitch_detection::detector::mcleod::McLeodDetector;
use pitch_detection::detector::PitchDetector as PitchDetectorTrait;
use serde::{Deserialize, Serialize};

/// Event emitted when pitch is detected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchEvent {
    /// Timestamp in milliseconds since game start.
    pub timestamp_ms: u64,
    /// Detected pitch in Hz (None if no pitch detected).
    pub pitch_hz: Option<f64>,
    /// Confidence/clarity of the detection [0.0, 1.0].
    pub confidence: f64,
}

/// Configuration for pitch detection.
#[derive(Debug, Clone)]
pub struct PitchDetectorConfig {
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Window size in samples (should be power of 2).
    pub window_size: usize,
    /// Padding factor for FFT.
    pub padding: usize,
    /// Minimum frequency to detect (Hz).
    pub min_freq: f32,
    /// Maximum frequency to detect (Hz).
    pub max_freq: f32,
    /// Minimum clarity threshold [0.0, 1.0].
    pub clarity_threshold: f32,
}

impl Default for PitchDetectorConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            window_size: 2048,  // ~46ms at 44.1kHz
            padding: 2048,     // Additional padding for FFT
            min_freq: 65.0,    // C2
            max_freq: 1047.0,  // C6
            clarity_threshold: 0.5,
        }
    }
}

/// Real-time pitch detector using McLeod algorithm.
pub struct PitchDetector {
    config: PitchDetectorConfig,
}

impl PitchDetector {
    /// Create a new pitch detector with default configuration.
    pub fn new() -> Self {
        Self::with_config(PitchDetectorConfig::default())
    }

    /// Create a new pitch detector with custom configuration.
    pub fn with_config(config: PitchDetectorConfig) -> Self {
        Self { config }
    }

    /// Detect pitch from a buffer of samples.
    /// Returns None if no pitch is detected or confidence is too low.
    pub fn detect(&self, samples: &[f32]) -> Option<(f64, f64)> {
        if samples.len() < self.config.window_size {
            return None;
        }

        let mut detector = McLeodDetector::new(self.config.window_size, self.config.padding);

        let pitch = detector.get_pitch(
            samples,
            self.config.sample_rate as usize,
            0.0,  // power_threshold
            self.config.clarity_threshold,
        )?;

        let freq = pitch.frequency as f64;

        // Filter out frequencies outside our range
        if freq < self.config.min_freq as f64 || freq > self.config.max_freq as f64 {
            return None;
        }

        Some((freq, pitch.clarity as f64))
    }

    /// Detect pitch and create a PitchEvent.
    pub fn detect_event(&self, samples: &[f32], timestamp_ms: u64) -> PitchEvent {
        match self.detect(samples) {
            Some((freq, clarity)) => PitchEvent {
                timestamp_ms,
                pitch_hz: Some(freq),
                confidence: clarity,
            },
            None => PitchEvent {
                timestamp_ms,
                pitch_hz: None,
                confidence: 0.0,
            },
        }
    }

    /// Get the required window size.
    pub fn window_size(&self) -> usize {
        self.config.window_size
    }

    /// Get the sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    /// Calculate hop size for 50Hz detection rate.
    pub fn hop_size_for_50hz(&self) -> usize {
        self.config.sample_rate as usize / 50
    }
}

impl Default for PitchDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert frequency in Hz to MIDI note number.
pub fn hz_to_midi(freq_hz: f64) -> f64 {
    69.0 + 12.0 * (freq_hz / 440.0).log2()
}

/// Convert MIDI note number to frequency in Hz.
pub fn midi_to_hz(midi: f64) -> f64 {
    440.0 * 2.0_f64.powf((midi - 69.0) / 12.0)
}

/// Calculate cents deviation between two frequencies.
/// Positive = sharp, Negative = flat.
pub fn cents_deviation(actual_hz: f64, target_hz: f64) -> f64 {
    1200.0 * (actual_hz / target_hz).log2()
}

/// Get the closest note name for a frequency.
pub fn hz_to_note_name(freq_hz: f64) -> String {
    const NOTE_NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];

    let midi = hz_to_midi(freq_hz).round() as i32;
    let note_idx = ((midi % 12) + 12) % 12;
    let octave = (midi / 12) - 1;

    format!("{}{}", NOTE_NAMES[note_idx as usize], octave)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn generate_sine(freq_hz: f32, sample_rate: u32, samples: usize) -> Vec<f32> {
        (0..samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * freq_hz * t).sin()
            })
            .collect()
    }

    #[test]
    fn test_detect_a440() {
        let detector = PitchDetector::new();
        let samples = generate_sine(440.0, 44100, 4096);

        let result = detector.detect(&samples);
        assert!(result.is_some());

        let (freq, clarity) = result.unwrap();
        assert!(
            (freq - 440.0).abs() < 5.0,
            "Expected ~440Hz, got {}",
            freq
        );
        assert!(clarity > 0.8, "Expected high clarity, got {}", clarity);
    }

    #[test]
    fn test_hz_to_midi() {
        assert!((hz_to_midi(440.0) - 69.0).abs() < 0.01);
        assert!((hz_to_midi(261.63) - 60.0).abs() < 0.1); // C4
    }

    #[test]
    fn test_cents_deviation() {
        // Same frequency = 0 cents
        assert!((cents_deviation(440.0, 440.0)).abs() < 0.01);

        // One semitone sharp = +100 cents
        let semitone_up = 440.0 * 2.0_f64.powf(1.0 / 12.0);
        assert!((cents_deviation(semitone_up, 440.0) - 100.0).abs() < 1.0);

        // One semitone flat = -100 cents
        let semitone_down = 440.0 * 2.0_f64.powf(-1.0 / 12.0);
        assert!((cents_deviation(semitone_down, 440.0) + 100.0).abs() < 1.0);
    }

    #[test]
    fn test_hz_to_note_name() {
        assert_eq!(hz_to_note_name(440.0), "A4");
        assert_eq!(hz_to_note_name(261.63), "C4");
        assert_eq!(hz_to_note_name(329.63), "E4");
    }
}
