//! MIDI file import for vocal note charts.
//!
//! Allows importing pre-made MIDI files containing vocal melodies.
//! The MIDI notes are converted to the same pitch contour format
//! used by the automatic analysis.

use anyhow::{Context, Result, bail};
use midly::{Smf, TrackEventKind, MidiMessage};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::pitch_contour::PitchContourData;

/// Configuration for MIDI import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiImportConfig {
    /// Track number to use for vocals (0-indexed, or None to auto-detect)
    pub track_index: Option<usize>,
    /// Track name to search for (e.g., "Vocals", "Lead Vocal", "Melody")
    pub track_name: Option<String>,
    /// Time resolution for output (samples per second)
    pub output_sample_rate: u32,
    /// Hop size in samples for output frames
    pub hop_length: usize,
}

impl Default for MidiImportConfig {
    fn default() -> Self {
        Self {
            track_index: None,
            track_name: None,
            output_sample_rate: 44100,
            hop_length: 512,
        }
    }
}

/// Convert MIDI note number to frequency in Hz.
pub fn midi_to_hz(midi_note: u8) -> f64 {
    440.0 * 2.0_f64.powf((midi_note as f64 - 69.0) / 12.0)
}

/// A note event extracted from MIDI.
#[derive(Debug, Clone)]
struct NoteEvent {
    /// Start time in seconds
    start_time_s: f64,
    /// End time in seconds
    end_time_s: f64,
    /// MIDI note number
    midi_note: u8,
    /// Velocity (0-127)
    velocity: u8,
}

/// Import a MIDI file and convert to pitch contour data.
pub fn import_midi_to_pitch_contour(
    midi_path: &Path,
    duration_seconds: f64,
    config: &MidiImportConfig,
) -> Result<PitchContourData> {
    let midi_data = std::fs::read(midi_path)
        .with_context(|| format!("Failed to read MIDI file: {}", midi_path.display()))?;

    let smf = Smf::parse(&midi_data)
        .with_context(|| "Failed to parse MIDI file")?;

    // Get timing information
    let ticks_per_beat = match smf.header.timing {
        midly::Timing::Metrical(tpb) => tpb.as_int() as f64,
        midly::Timing::Timecode(fps, sub) => {
            // Convert timecode to approximate ticks per beat
            (fps.as_f32() * sub as f32) as f64
        }
    };

    // Default tempo (120 BPM) - will be updated if tempo events found
    let mut microseconds_per_beat: f64 = 500_000.0; // 120 BPM

    // Find the track to use
    let track_idx = find_vocal_track(&smf, config)?;
    let track = &smf.tracks[track_idx];

    // Extract note events
    let mut notes: Vec<NoteEvent> = Vec::new();
    let mut current_tick: u64 = 0;
    let mut active_notes: std::collections::HashMap<u8, (f64, u8)> = std::collections::HashMap::new();

    for event in track.iter() {
        current_tick += event.delta.as_int() as u64;
        let current_time_s = ticks_to_seconds(current_tick, ticks_per_beat, microseconds_per_beat);

        match event.kind {
            TrackEventKind::Meta(midly::MetaMessage::Tempo(tempo)) => {
                microseconds_per_beat = tempo.as_int() as f64;
            }
            TrackEventKind::Midi { message, .. } => {
                match message {
                    MidiMessage::NoteOn { key, vel } => {
                        if vel.as_int() > 0 {
                            active_notes.insert(key.as_int(), (current_time_s, vel.as_int()));
                        } else {
                            // Note on with velocity 0 is note off
                            if let Some((start_time, velocity)) = active_notes.remove(&key.as_int()) {
                                notes.push(NoteEvent {
                                    start_time_s: start_time,
                                    end_time_s: current_time_s,
                                    midi_note: key.as_int(),
                                    velocity,
                                });
                            }
                        }
                    }
                    MidiMessage::NoteOff { key, .. } => {
                        if let Some((start_time, velocity)) = active_notes.remove(&key.as_int()) {
                            notes.push(NoteEvent {
                                start_time_s: start_time,
                                end_time_s: current_time_s,
                                midi_note: key.as_int(),
                                velocity,
                            });
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Convert note events to frame-based pitch contour
    let hop_duration_s = config.hop_length as f64 / config.output_sample_rate as f64;
    let num_frames = (duration_seconds / hop_duration_s).ceil() as usize;

    let mut times_s = Vec::with_capacity(num_frames);
    let mut frequencies_hz = Vec::with_capacity(num_frames);
    let mut voiced = Vec::with_capacity(num_frames);
    let mut voiced_prob = Vec::with_capacity(num_frames);

    for i in 0..num_frames {
        let frame_time = i as f64 * hop_duration_s;
        times_s.push(frame_time);

        // Find if any note is active at this time
        let active_note = notes.iter().find(|n| {
            n.start_time_s <= frame_time && frame_time < n.end_time_s
        });

        match active_note {
            Some(note) => {
                let freq = midi_to_hz(note.midi_note);
                frequencies_hz.push(Some(freq));
                voiced.push(true);
                voiced_prob.push(note.velocity as f64 / 127.0);
            }
            None => {
                frequencies_hz.push(None);
                voiced.push(false);
                voiced_prob.push(0.0);
            }
        }
    }

    Ok(PitchContourData {
        times_s,
        frequencies_hz,
        voiced,
        voiced_prob,
    })
}

/// Find the vocal track in the MIDI file.
fn find_vocal_track(smf: &Smf, config: &MidiImportConfig) -> Result<usize> {
    // If track index specified, use it
    if let Some(idx) = config.track_index {
        if idx < smf.tracks.len() {
            return Ok(idx);
        }
        bail!("Track index {} out of range (file has {} tracks)", idx, smf.tracks.len());
    }

    // Search by track name
    let search_names: Vec<&str> = if let Some(ref name) = config.track_name {
        vec![name.as_str()]
    } else {
        vec!["vocal", "vocals", "voice", "lead", "melody", "singer", "vox"]
    };

    for (idx, track) in smf.tracks.iter().enumerate() {
        for event in track.iter() {
            if let TrackEventKind::Meta(midly::MetaMessage::TrackName(name)) = event.kind {
                let track_name = String::from_utf8_lossy(name).to_lowercase();
                for search in &search_names {
                    if track_name.contains(&search.to_lowercase()) {
                        println!("Found vocal track: '{}' at index {}", track_name, idx);
                        return Ok(idx);
                    }
                }
            }
        }
    }

    // If no named track found, find the track with the most notes in vocal range
    let mut best_track = 0;
    let mut best_vocal_notes = 0;

    for (idx, track) in smf.tracks.iter().enumerate() {
        let mut vocal_notes = 0;
        for event in track.iter() {
            if let TrackEventKind::Midi { message: MidiMessage::NoteOn { key, vel }, .. } = event.kind {
                if vel.as_int() > 0 {
                    let note = key.as_int();
                    // Vocal range: roughly C3 (48) to C6 (84)
                    if note >= 48 && note <= 84 {
                        vocal_notes += 1;
                    }
                }
            }
        }
        if vocal_notes > best_vocal_notes {
            best_vocal_notes = vocal_notes;
            best_track = idx;
        }
    }

    if best_vocal_notes > 0 {
        println!("Auto-selected track {} with {} notes in vocal range", best_track, best_vocal_notes);
        Ok(best_track)
    } else {
        // Fall back to first non-empty track
        for (idx, track) in smf.tracks.iter().enumerate() {
            if !track.is_empty() {
                return Ok(idx);
            }
        }
        bail!("No suitable track found in MIDI file");
    }
}

/// Convert MIDI ticks to seconds.
fn ticks_to_seconds(ticks: u64, ticks_per_beat: f64, microseconds_per_beat: f64) -> f64 {
    let beats = ticks as f64 / ticks_per_beat;
    beats * microseconds_per_beat / 1_000_000.0
}

/// List all tracks in a MIDI file with their names and note counts.
pub fn list_midi_tracks(midi_path: &Path) -> Result<Vec<(usize, String, usize)>> {
    let midi_data = std::fs::read(midi_path)?;
    let smf = Smf::parse(&midi_data)?;

    let mut tracks = Vec::new();

    for (idx, track) in smf.tracks.iter().enumerate() {
        let mut name = format!("Track {}", idx);
        let mut note_count = 0;

        for event in track.iter() {
            match event.kind {
                TrackEventKind::Meta(midly::MetaMessage::TrackName(n)) => {
                    name = String::from_utf8_lossy(n).to_string();
                }
                TrackEventKind::Midi { message: MidiMessage::NoteOn { vel, .. }, .. } => {
                    if vel.as_int() > 0 {
                        note_count += 1;
                    }
                }
                _ => {}
            }
        }

        tracks.push((idx, name, note_count));
    }

    Ok(tracks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_to_hz() {
        assert!((midi_to_hz(69) - 440.0).abs() < 0.01); // A4
        assert!((midi_to_hz(60) - 261.63).abs() < 0.1); // C4
        assert!((midi_to_hz(48) - 130.81).abs() < 0.1); // C3
    }
}
