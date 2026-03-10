import { useState, useEffect } from 'react';
import { GameSessionResult } from '../../stores/gameStore';
import { formatScore } from '../../utils/pitchMath';

interface VersusScreenProps {
  player1Result: GameSessionResult;
  player2Result: GameSessionResult;
  onPlayAgain: () => void;
  onBack: () => void;
}

/**
 * VS comparison screen showing both players' scores.
 */
export default function VersusScreen({
  player1Result,
  player2Result,
  onPlayAgain,
  onBack,
}: VersusScreenProps) {
  const [showWinner, setShowWinner] = useState(false);
  const [animationPhase, setAnimationPhase] = useState(0);

  useEffect(() => {
    // Animate the reveal
    const timers = [
      setTimeout(() => setAnimationPhase(1), 500),  // Show P1 score
      setTimeout(() => setAnimationPhase(2), 1500), // Show P2 score
      setTimeout(() => setShowWinner(true), 2500),  // Show winner
    ];

    return () => timers.forEach(clearTimeout);
  }, []);

  const winner = player1Result.score > player2Result.score
    ? 'player1'
    : player1Result.score < player2Result.score
      ? 'player2'
      : 'tie';

  return (
    <div style={styles.container}>
      <h1 style={styles.vsTitle}>VS</h1>

      <div style={styles.playersContainer}>
        {/* Player 1 */}
        <div
          style={{
            ...styles.playerCard,
            ...styles.player1Card,
            opacity: animationPhase >= 1 ? 1 : 0,
            transform: animationPhase >= 1 ? 'translateX(0)' : 'translateX(-100px)',
          }}
        >
          <div style={styles.playerName}>{player1Result.player_name}</div>
          <div style={{ ...styles.playerScore, color: '#4A90D9' }}>
            {formatScore(player1Result.score)}
          </div>
          <div style={styles.playerAccuracy}>
            {player1Result.accuracy_pct.toFixed(1)}% Accuracy
          </div>
          <div style={styles.playerStreak}>
            Max Streak: {player1Result.max_streak}
          </div>
        </div>

        {/* VS divider */}
        <div style={styles.vsDivider}>
          <span style={styles.vsText}>VS</span>
        </div>

        {/* Player 2 */}
        <div
          style={{
            ...styles.playerCard,
            ...styles.player2Card,
            opacity: animationPhase >= 2 ? 1 : 0,
            transform: animationPhase >= 2 ? 'translateX(0)' : 'translateX(100px)',
          }}
        >
          <div style={styles.playerName}>{player2Result.player_name}</div>
          <div style={{ ...styles.playerScore, color: '#D94A4A' }}>
            {formatScore(player2Result.score)}
          </div>
          <div style={styles.playerAccuracy}>
            {player2Result.accuracy_pct.toFixed(1)}% Accuracy
          </div>
          <div style={styles.playerStreak}>
            Max Streak: {player2Result.max_streak}
          </div>
        </div>
      </div>

      {/* Winner announcement */}
      {showWinner && (
        <div
          style={{
            ...styles.winnerContainer,
            animation: 'winnerPop 0.5s ease-out',
          }}
        >
          {winner === 'tie' ? (
            <div style={styles.tieText}>It's a TIE!</div>
          ) : (
            <>
              <div style={styles.winnerLabel}>WINNER</div>
              <div
                style={{
                  ...styles.winnerName,
                  color: winner === 'player1' ? '#4A90D9' : '#D94A4A',
                }}
              >
                {winner === 'player1' ? player1Result.player_name : player2Result.player_name}
              </div>
            </>
          )}
        </div>
      )}

      {/* Actions */}
      <div style={styles.actions}>
        <button onClick={onPlayAgain} style={styles.rematchButton}>
          Rematch
        </button>
        <button onClick={onBack} style={styles.backButton}>
          Back to Main
        </button>
      </div>

      <style>{`
        @keyframes winnerPop {
          0% { transform: scale(0.5); opacity: 0; }
          50% { transform: scale(1.2); }
          100% { transform: scale(1); opacity: 1; }
        }
      `}</style>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    justifyContent: 'center',
    height: '100%',
    padding: 32,
    backgroundColor: '#0a0a15',
    color: 'white',
  },
  vsTitle: {
    fontSize: 64,
    fontWeight: 'bold',
    marginBottom: 32,
    background: 'linear-gradient(45deg, #4A90D9, #D94A4A)',
    WebkitBackgroundClip: 'text',
    WebkitTextFillColor: 'transparent',
    textShadow: 'none',
  },
  playersContainer: {
    display: 'flex',
    alignItems: 'center',
    gap: 48,
    marginBottom: 48,
  },
  playerCard: {
    width: 280,
    padding: 32,
    borderRadius: 16,
    textAlign: 'center',
    transition: 'all 0.5s ease-out',
  },
  player1Card: {
    backgroundColor: 'rgba(74, 144, 217, 0.1)',
    border: '2px solid #4A90D9',
  },
  player2Card: {
    backgroundColor: 'rgba(217, 74, 74, 0.1)',
    border: '2px solid #D94A4A',
  },
  playerName: {
    fontSize: 24,
    fontWeight: 'bold',
    marginBottom: 16,
  },
  playerScore: {
    fontSize: 48,
    fontWeight: 'bold',
    marginBottom: 8,
  },
  playerAccuracy: {
    fontSize: 16,
    color: '#888888',
    marginBottom: 4,
  },
  playerStreak: {
    fontSize: 14,
    color: '#666666',
  },
  vsDivider: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
  },
  vsText: {
    fontSize: 48,
    fontWeight: 'bold',
    color: '#444444',
  },
  winnerContainer: {
    textAlign: 'center',
    marginBottom: 48,
  },
  winnerLabel: {
    fontSize: 24,
    color: '#888888',
    marginBottom: 8,
  },
  winnerName: {
    fontSize: 48,
    fontWeight: 'bold',
    textShadow: '0 0 30px currentColor',
  },
  tieText: {
    fontSize: 48,
    fontWeight: 'bold',
    color: '#FFD700',
    textShadow: '0 0 30px #FFD700',
  },
  actions: {
    display: 'flex',
    gap: 16,
  },
  rematchButton: {
    padding: '16px 48px',
    fontSize: 20,
    fontWeight: 'bold',
    backgroundColor: '#4A90D9',
    color: 'white',
    border: 'none',
    borderRadius: 8,
    cursor: 'pointer',
  },
  backButton: {
    padding: '16px 32px',
    fontSize: 16,
    backgroundColor: 'transparent',
    color: '#888888',
    border: '1px solid #444444',
    borderRadius: 8,
    cursor: 'pointer',
  },
};
