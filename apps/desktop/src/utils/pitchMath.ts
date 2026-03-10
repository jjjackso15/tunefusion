// Pitch math utilities for the game

const NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'] as const;

/**
 * Convert frequency in Hz to MIDI note number.
 */
export function hzToMidi(freqHz: number): number {
  return 69 + 12 * Math.log2(freqHz / 440);
}

/**
 * Convert MIDI note number to frequency in Hz.
 */
export function midiToHz(midi: number): number {
  return 440 * Math.pow(2, (midi - 69) / 12);
}

/**
 * Calculate cents deviation between two frequencies.
 * Positive = sharp, Negative = flat.
 */
export function centsDeviation(actualHz: number, targetHz: number): number {
  return 1200 * Math.log2(actualHz / targetHz);
}

/**
 * Get the note name for a frequency.
 */
export function hzToNoteName(freqHz: number): string {
  const midi = Math.round(hzToMidi(freqHz));
  const noteIdx = ((midi % 12) + 12) % 12;
  const octave = Math.floor(midi / 12) - 1;
  return `${NOTE_NAMES[noteIdx]}${octave}`;
}

/**
 * Get the note name and octave separately.
 */
export function hzToNoteInfo(freqHz: number): { note: string; octave: number } {
  const midi = Math.round(hzToMidi(freqHz));
  const noteIdx = ((midi % 12) + 12) % 12;
  const octave = Math.floor(midi / 12) - 1;
  return { note: NOTE_NAMES[noteIdx], octave };
}

/**
 * Convert frequency to Y position on a log scale.
 * Returns a value between 0 (low) and 1 (high).
 */
export function hzToLogY(freqHz: number, minHz: number = 65, maxHz: number = 1047): number {
  const logMin = Math.log2(minHz);
  const logMax = Math.log2(maxHz);
  const logFreq = Math.log2(freqHz);
  return (logFreq - logMin) / (logMax - logMin);
}

/**
 * Convert Y position (0-1) back to frequency on log scale.
 */
export function logYToHz(y: number, minHz: number = 65, maxHz: number = 1047): number {
  const logMin = Math.log2(minHz);
  const logMax = Math.log2(maxHz);
  return Math.pow(2, logMin + y * (logMax - logMin));
}

/**
 * Clamp a value between min and max.
 */
export function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

/**
 * Linear interpolation between two values.
 */
export function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

/**
 * Smooth step interpolation.
 */
export function smoothstep(edge0: number, edge1: number, x: number): number {
  const t = clamp((x - edge0) / (edge1 - edge0), 0, 1);
  return t * t * (3 - 2 * t);
}

/**
 * Get rating color based on hit rating.
 */
export function getRatingColor(rating: string | null): string {
  switch (rating) {
    case 'perfect': return '#FFD700'; // Gold
    case 'great': return '#00FF00';   // Green
    case 'good': return '#00BFFF';    // Blue
    case 'ok': return '#FFA500';      // Orange
    case 'miss': return '#FF4444';    // Red
    default: return '#FFFFFF';
  }
}

/**
 * Get rating glow intensity.
 */
export function getRatingGlowIntensity(rating: string | null): number {
  switch (rating) {
    case 'perfect': return 1.0;
    case 'great': return 0.8;
    case 'good': return 0.6;
    case 'ok': return 0.4;
    case 'miss': return 0.2;
    default: return 0;
  }
}

/**
 * Player colors.
 */
export const PLAYER_COLORS = {
  player1: {
    primary: '#4A90D9',
    secondary: '#2E5A8A',
    glow: '#6BB3FF',
  },
  player2: {
    primary: '#D94A4A',
    secondary: '#8A2E2E',
    glow: '#FF6B6B',
  },
};

/**
 * Format time in MM:SS format.
 */
export function formatTime(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, '0')}`;
}

/**
 * Format score with commas.
 */
export function formatScore(score: number): string {
  return score.toLocaleString();
}
