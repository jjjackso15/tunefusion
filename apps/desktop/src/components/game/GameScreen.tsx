import { useEffect, useState, lazy, Suspense } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useGameStore, GameMode, TrackInfo, GameSessionResult } from '../../stores/gameStore';
import { useGameLoop } from '../../hooks/useGameLoop';
import { formatTime, formatScore } from '../../utils/pitchMath';
import ModeSelector from './ModeSelector';
import PlayerSelector from './PlayerSelector';
import LeaderboardView from './LeaderboardView';
import ResultsScreen from './ResultsScreen';

// Lazy load GameRenderer to defer WebGL initialization
const GameRenderer = lazy(() => import('./GameRenderer'));

interface GameScreenProps {
  tracks: TrackInfo[];
  onBack: () => void;
}

export default function GameScreen({ tracks, onBack }: GameScreenProps) {
  const {
    gameState,
    gameMode,
    selectedTrack,
    currentPlayer,
    countdown,
    playbackPosition,
    playbackDuration,
    score,
    setGameState,
    setGameMode,
    setSelectedTrack,
    setCurrentPlayer,
    setTargetPitches,
    addResult,
    reset,
  } = useGameStore();

  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  // Initialize game loop event listeners
  useGameLoop();

  // Handle mode selection
  const handleModeSelect = (mode: GameMode) => {
    setGameMode(mode);
    setGameState('player_select');
  };

  // Handle player selection
  const handlePlayerSelect = (name: string) => {
    setCurrentPlayer({ name, color: '#4A90D9' });
    setGameState('ready');
  };

  // Handle track selection
  const handleTrackSelect = (track: TrackInfo) => {
    setSelectedTrack(track);
  };

  // Start the game
  const handleStartGame = async () => {
    if (!selectedTrack || !currentPlayer || !gameMode) {
      setError('Please select a track and enter player name');
      return;
    }

    setError(null);
    setLoading(true);

    try {
      await invoke('start_game', {
        trackId: selectedTrack.id,
        playerName: currentPlayer.name,
        mode: gameMode,
      });

      // Begin countdown
      await invoke('begin_countdown');
    } catch (e) {
      setError(String(e));
      setLoading(false);
    }
  };

  // Pause game
  const handlePause = async () => {
    try {
      await invoke('pause_game');
    } catch (e) {
      setError(String(e));
    }
  };

  // Resume game
  const handleResume = async () => {
    try {
      await invoke('resume_game');
    } catch (e) {
      setError(String(e));
    }
  };

  // Stop game and get results
  const handleStop = async () => {
    try {
      const result = await invoke<GameSessionResult | null>('stop_game');
      if (result) {
        addResult(result);
      }
      setGameState('results');
    } catch (e) {
      setError(String(e));
    }
  };

  // Play again
  const handlePlayAgain = () => {
    reset();
    setGameState('mode_select');
  };

  // Back to main
  const handleBackToMain = () => {
    reset();
    onBack();
  };

  // Render based on game state
  const renderContent = () => {
    switch (gameState) {
      case 'idle':
      case 'mode_select':
        return (
          <ModeSelector
            onSelect={handleModeSelect}
            onBack={handleBackToMain}
          />
        );

      case 'player_select':
        return (
          <PlayerSelector
            tracks={tracks}
            selectedTrack={selectedTrack}
            onTrackSelect={handleTrackSelect}
            onPlayerSubmit={handlePlayerSelect}
            onBack={() => setGameState('mode_select')}
          />
        );

      case 'ready':
        return (
          <div style={styles.centerContainer}>
            <h2>Ready to Play!</h2>
            {selectedTrack && (
              <p style={styles.trackInfo}>
                {selectedTrack.title} ({formatTime(selectedTrack.duration_seconds * 1000)})
              </p>
            )}
            <p style={styles.playerName}>Player: {currentPlayer?.name}</p>
            <p style={styles.modeLabel}>Mode: {gameMode === 'solo' ? 'Solo Practice' : 'Competition'}</p>

            <button
              onClick={handleStartGame}
              disabled={loading}
              style={styles.startButton}
            >
              {loading ? 'Starting...' : 'Start!'}
            </button>

            <button onClick={() => setGameState('player_select')} style={styles.backButton}>
              Back
            </button>
          </div>
        );

      case 'countdown':
        return (
          <div style={styles.countdownContainer}>
            <div style={styles.countdownNumber}>{countdown}</div>
            <p style={styles.countdownText}>Get Ready!</p>
          </div>
        );

      case 'playing':
        return (
          <div style={styles.gameContainer}>
            <Suspense fallback={<div style={styles.loading}>Loading renderer...</div>}>
              <GameRenderer />
            </Suspense>

            {/* Progress bar */}
            <div style={styles.progressContainer}>
              <div
                style={{
                  ...styles.progressBar,
                  width: `${(playbackPosition / playbackDuration) * 100}%`,
                }}
              />
            </div>

            {/* Time display */}
            <div style={styles.timeDisplay}>
              {formatTime(playbackPosition)} / {formatTime(playbackDuration)}
            </div>

            {/* Pause button */}
            <button onClick={handlePause} style={styles.pauseButton}>
              Pause
            </button>
          </div>
        );

      case 'paused':
        return (
          <div style={styles.pauseOverlay}>
            <h2>Paused</h2>
            <button onClick={handleResume} style={styles.resumeButton}>
              Resume
            </button>
            <button onClick={handleStop} style={styles.quitButton}>
              Quit
            </button>
          </div>
        );

      case 'finished':
      case 'results':
        return (
          <ResultsScreen
            onPlayAgain={handlePlayAgain}
            onBack={handleBackToMain}
          />
        );

      default:
        return <div>Unknown state: {gameState}</div>;
    }
  };

  return (
    <div style={styles.container}>
      {error && <div style={styles.error}>{error}</div>}
      {renderContent()}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    width: '100%',
    height: '100vh',
    backgroundColor: '#0a0a15',
    color: 'white',
    fontFamily: 'system-ui, sans-serif',
  },
  centerContainer: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    padding: 32,
  },
  trackInfo: {
    fontSize: 24,
    color: '#88AAFF',
    marginBottom: 8,
  },
  playerName: {
    fontSize: 20,
    color: '#AAAAAA',
    marginBottom: 8,
  },
  modeLabel: {
    fontSize: 18,
    color: '#888888',
    marginBottom: 32,
  },
  startButton: {
    padding: '16px 48px',
    fontSize: 24,
    fontWeight: 'bold',
    backgroundColor: '#4A90D9',
    color: 'white',
    border: 'none',
    borderRadius: 8,
    cursor: 'pointer',
    marginBottom: 16,
  },
  backButton: {
    padding: '8px 24px',
    fontSize: 16,
    backgroundColor: 'transparent',
    color: '#888888',
    border: '1px solid #444444',
    borderRadius: 4,
    cursor: 'pointer',
  },
  countdownContainer: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
  },
  countdownNumber: {
    fontSize: 200,
    fontWeight: 'bold',
    color: '#FFD700',
    textShadow: '0 0 50px #FFD700',
    animation: 'pulse 1s ease-in-out infinite',
  },
  countdownText: {
    fontSize: 32,
    color: '#888888',
  },
  gameContainer: {
    width: '100%',
    height: '100%',
    position: 'relative',
  },
  progressContainer: {
    position: 'absolute',
    bottom: 0,
    left: 0,
    right: 0,
    height: 4,
    backgroundColor: '#333333',
  },
  progressBar: {
    height: '100%',
    backgroundColor: '#4A90D9',
    transition: 'width 0.1s linear',
  },
  timeDisplay: {
    position: 'absolute',
    bottom: 16,
    left: 16,
    fontSize: 14,
    color: '#888888',
  },
  pauseButton: {
    position: 'absolute',
    top: 16,
    left: 16,
    padding: '8px 16px',
    fontSize: 14,
    backgroundColor: 'rgba(0,0,0,0.5)',
    color: 'white',
    border: '1px solid #444444',
    borderRadius: 4,
    cursor: 'pointer',
  },
  pauseOverlay: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    backgroundColor: 'rgba(0,0,0,0.8)',
  },
  resumeButton: {
    padding: '16px 48px',
    fontSize: 20,
    backgroundColor: '#4A90D9',
    color: 'white',
    border: 'none',
    borderRadius: 8,
    cursor: 'pointer',
    marginBottom: 16,
  },
  quitButton: {
    padding: '8px 24px',
    fontSize: 16,
    backgroundColor: 'transparent',
    color: '#FF6666',
    border: '1px solid #FF6666',
    borderRadius: 4,
    cursor: 'pointer',
  },
  error: {
    position: 'absolute',
    top: 16,
    left: '50%',
    transform: 'translateX(-50%)',
    padding: '8px 16px',
    backgroundColor: 'rgba(255,0,0,0.8)',
    borderRadius: 4,
    zIndex: 100,
  },
  loading: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    color: '#888888',
    fontSize: 20,
  },
};
