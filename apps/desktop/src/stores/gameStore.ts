import { create } from 'zustand';

// Game mode types
export type GameMode = 'solo' | 'competition';
export type GameState = 'idle' | 'mode_select' | 'player_select' | 'ready' | 'countdown' | 'playing' | 'paused' | 'finished' | 'results';

// Hit rating types
export type HitRating = 'perfect' | 'great' | 'good' | 'ok' | 'miss';

// Pitch event from backend
export interface PitchEvent {
  timestamp_ms: number;
  pitch_hz: number | null;
  confidence: number;
}

// Score update from backend
export interface ScoreUpdate {
  score: number;
  streak: number;
  max_streak: number;
  accuracy_pct: number;
  last_rating: HitRating | null;
  perfect_count: number;
  great_count: number;
  good_count: number;
  ok_count: number;
  miss_count: number;
}

// Playback tick from backend
export interface PlaybackTick {
  position_ms: number;
  is_playing: boolean;
  duration_ms: number;
}

// Target pitch data
export interface TargetPitch {
  time_ms: number;
  frequency_hz: number | null;
  voiced: boolean;
}

// Player info
export interface PlayerInfo {
  name: string;
  color: string;
}

// Game session result
export interface GameSessionResult {
  id: string;
  track_id: string;
  player_name: string;
  mode: GameMode;
  score: number;
  accuracy_pct: number;
  perfect_count: number;
  great_count: number;
  good_count: number;
  ok_count: number;
  miss_count: number;
  max_streak: number;
}

// Track info
export interface TrackInfo {
  id: string;
  title: string;
  duration_seconds: number;
}

interface GameStore {
  // State
  gameState: GameState;
  gameMode: GameMode | null;
  selectedTrack: TrackInfo | null;
  currentPlayer: PlayerInfo | null;
  players: PlayerInfo[];
  countdown: number | null;

  // Playback state
  playbackPosition: number;
  playbackDuration: number;
  isPlaying: boolean;

  // Pitch detection
  userPitch: number | null;
  userConfidence: number;
  targetPitches: TargetPitch[];

  // Scoring
  score: number;
  streak: number;
  maxStreak: number;
  accuracyPct: number;
  lastRating: HitRating | null;
  perfectCount: number;
  greatCount: number;
  goodCount: number;
  okCount: number;
  missCount: number;

  // Results
  results: GameSessionResult[];

  // Visual effects
  showHitEffect: boolean;
  hitEffectRating: HitRating | null;

  // Actions
  setGameState: (state: GameState) => void;
  setGameMode: (mode: GameMode | null) => void;
  setSelectedTrack: (track: TrackInfo | null) => void;
  setCurrentPlayer: (player: PlayerInfo | null) => void;
  setPlayers: (players: PlayerInfo[]) => void;
  setCountdown: (countdown: number | null) => void;

  updatePlayback: (tick: PlaybackTick) => void;
  updateUserPitch: (event: PitchEvent) => void;
  updateScore: (update: ScoreUpdate) => void;
  setTargetPitches: (pitches: TargetPitch[]) => void;

  addResult: (result: GameSessionResult) => void;
  clearResults: () => void;

  triggerHitEffect: (rating: HitRating) => void;
  clearHitEffect: () => void;

  reset: () => void;
}

const initialState = {
  gameState: 'idle' as GameState,
  gameMode: null,
  selectedTrack: null,
  currentPlayer: null,
  players: [],
  countdown: null,

  playbackPosition: 0,
  playbackDuration: 0,
  isPlaying: false,

  userPitch: null,
  userConfidence: 0,
  targetPitches: [],

  score: 0,
  streak: 0,
  maxStreak: 0,
  accuracyPct: 100,
  lastRating: null,
  perfectCount: 0,
  greatCount: 0,
  goodCount: 0,
  okCount: 0,
  missCount: 0,

  results: [],

  showHitEffect: false,
  hitEffectRating: null,
};

export const useGameStore = create<GameStore>((set) => ({
  ...initialState,

  setGameState: (state) => set({ gameState: state }),
  setGameMode: (mode) => set({ gameMode: mode }),
  setSelectedTrack: (track) => set({ selectedTrack: track }),
  setCurrentPlayer: (player) => set({ currentPlayer: player }),
  setPlayers: (players) => set({ players }),
  setCountdown: (countdown) => set({ countdown }),

  updatePlayback: (tick) => set({
    playbackPosition: tick.position_ms,
    playbackDuration: tick.duration_ms,
    isPlaying: tick.is_playing,
  }),

  updateUserPitch: (event) => set({
    userPitch: event.pitch_hz,
    userConfidence: event.confidence,
  }),

  updateScore: (update) => set({
    score: update.score,
    streak: update.streak,
    maxStreak: update.max_streak,
    accuracyPct: update.accuracy_pct,
    lastRating: update.last_rating,
    perfectCount: update.perfect_count,
    greatCount: update.great_count,
    goodCount: update.good_count,
    okCount: update.ok_count,
    missCount: update.miss_count,
  }),

  setTargetPitches: (pitches) => set({ targetPitches: pitches }),

  addResult: (result) => set((state) => ({
    results: [...state.results, result],
  })),

  clearResults: () => set({ results: [] }),

  triggerHitEffect: (rating) => set({
    showHitEffect: true,
    hitEffectRating: rating,
  }),

  clearHitEffect: () => set({
    showHitEffect: false,
    hitEffectRating: null,
  }),

  reset: () => set(initialState),
}));
