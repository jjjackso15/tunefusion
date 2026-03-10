import { useGameStore } from '../../stores/gameStore';
import { formatScore, getRatingColor } from '../../utils/pitchMath';

interface ResultsScreenProps {
  onPlayAgain: () => void;
  onBack: () => void;
}

export default function ResultsScreen({ onPlayAgain, onBack }: ResultsScreenProps) {
  const {
    results,
    score,
    accuracyPct,
    maxStreak,
    perfectCount,
    greatCount,
    goodCount,
    okCount,
    missCount,
    currentPlayer,
    selectedTrack,
  } = useGameStore();

  // Get the latest result
  const latestResult = results.length > 0 ? results[results.length - 1] : null;

  // Calculate grade
  const getGrade = (accuracy: number): { grade: string; color: string } => {
    if (accuracy >= 95) return { grade: 'S', color: '#FFD700' };
    if (accuracy >= 90) return { grade: 'A', color: '#00FF00' };
    if (accuracy >= 80) return { grade: 'B', color: '#00BFFF' };
    if (accuracy >= 70) return { grade: 'C', color: '#FFA500' };
    if (accuracy >= 60) return { grade: 'D', color: '#FF6666' };
    return { grade: 'F', color: '#FF0000' };
  };

  const { grade, color: gradeColor } = getGrade(accuracyPct);

  return (
    <div style={styles.container}>
      <h1 style={styles.title}>Results</h1>

      {selectedTrack && (
        <p style={styles.trackName}>{selectedTrack.title}</p>
      )}

      {currentPlayer && (
        <p style={styles.playerName}>{currentPlayer.name}</p>
      )}

      {/* Grade */}
      <div style={{ ...styles.grade, color: gradeColor }}>
        {grade}
      </div>

      {/* Score */}
      <div style={styles.scoreContainer}>
        <div style={styles.scoreLabel}>Final Score</div>
        <div style={styles.scoreValue}>{formatScore(score)}</div>
      </div>

      {/* Stats grid */}
      <div style={styles.statsGrid}>
        <div style={styles.statItem}>
          <div style={styles.statValue}>{accuracyPct.toFixed(1)}%</div>
          <div style={styles.statLabel}>Accuracy</div>
        </div>
        <div style={styles.statItem}>
          <div style={styles.statValue}>{maxStreak}</div>
          <div style={styles.statLabel}>Max Streak</div>
        </div>
      </div>

      {/* Rating breakdown */}
      <div style={styles.ratingBreakdown}>
        <h3 style={styles.breakdownTitle}>Rating Breakdown</h3>
        <div style={styles.ratingGrid}>
          <RatingBar label="Perfect" count={perfectCount} color={getRatingColor('perfect')} />
          <RatingBar label="Great" count={greatCount} color={getRatingColor('great')} />
          <RatingBar label="Good" count={goodCount} color={getRatingColor('good')} />
          <RatingBar label="OK" count={okCount} color={getRatingColor('ok')} />
          <RatingBar label="Miss" count={missCount} color={getRatingColor('miss')} />
        </div>
      </div>

      {/* Actions */}
      <div style={styles.actions}>
        <button onClick={onPlayAgain} style={styles.playAgainButton}>
          Play Again
        </button>
        <button onClick={onBack} style={styles.backButton}>
          Back to Main
        </button>
      </div>
    </div>
  );
}

function RatingBar({ label, count, color }: { label: string; count: number; color: string }) {
  return (
    <div style={styles.ratingRow}>
      <span style={{ ...styles.ratingLabel, color }}>{label}</span>
      <div style={styles.ratingBarContainer}>
        <div
          style={{
            ...styles.ratingBarFill,
            width: `${Math.min(100, count * 2)}%`,
            backgroundColor: color,
          }}
        />
      </div>
      <span style={styles.ratingCount}>{count}</span>
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    height: '100%',
    padding: 32,
    overflowY: 'auto',
  },
  title: {
    fontSize: 48,
    fontWeight: 'bold',
    marginBottom: 8,
    color: '#FFFFFF',
  },
  trackName: {
    fontSize: 20,
    color: '#88AAFF',
    marginBottom: 4,
  },
  playerName: {
    fontSize: 16,
    color: '#888888',
    marginBottom: 24,
  },
  grade: {
    fontSize: 120,
    fontWeight: 'bold',
    textShadow: '0 0 30px currentColor',
    marginBottom: 16,
  },
  scoreContainer: {
    textAlign: 'center',
    marginBottom: 32,
  },
  scoreLabel: {
    fontSize: 14,
    color: '#888888',
    marginBottom: 4,
  },
  scoreValue: {
    fontSize: 48,
    fontWeight: 'bold',
    color: '#FFD700',
  },
  statsGrid: {
    display: 'flex',
    gap: 48,
    marginBottom: 32,
  },
  statItem: {
    textAlign: 'center',
  },
  statValue: {
    fontSize: 32,
    fontWeight: 'bold',
    color: '#FFFFFF',
  },
  statLabel: {
    fontSize: 14,
    color: '#888888',
  },
  ratingBreakdown: {
    width: '100%',
    maxWidth: 400,
    marginBottom: 32,
  },
  breakdownTitle: {
    fontSize: 16,
    color: '#888888',
    marginBottom: 16,
    textAlign: 'center',
  },
  ratingGrid: {
    display: 'flex',
    flexDirection: 'column',
    gap: 8,
  },
  ratingRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 12,
  },
  ratingLabel: {
    width: 60,
    fontSize: 14,
    fontWeight: 'bold',
    textAlign: 'right',
  },
  ratingBarContainer: {
    flex: 1,
    height: 12,
    backgroundColor: '#1a1a2e',
    borderRadius: 6,
    overflow: 'hidden',
  },
  ratingBarFill: {
    height: '100%',
    borderRadius: 6,
    transition: 'width 0.5s ease',
  },
  ratingCount: {
    width: 40,
    fontSize: 14,
    color: '#AAAAAA',
    textAlign: 'left',
  },
  actions: {
    display: 'flex',
    gap: 16,
  },
  playAgainButton: {
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
