//! Game engine for TuneFusion's SingStar/Guitar Hero-style game mode.
//!
//! Provides real-time audio playback, microphone capture, pitch detection,
//! and scoring for singing games.

pub mod audio_player;
pub mod mic_capture;
pub mod pitch_detector;
pub mod scoring;

pub use audio_player::{AudioPlayer, PlaybackState, PlaybackTick};
pub use mic_capture::{MicCapture, MicConfig};
pub use pitch_detector::{PitchDetector, PitchEvent};
pub use scoring::{HitRating, ScoreUpdate, ScoringEngine};
