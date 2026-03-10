import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { formatScore } from '../../utils/pitchMath';

interface LeaderboardEntry {
  id: string;
  track_id: string;
  player_name: string;
  score: number;
  created_at: string;
}

interface LeaderboardViewProps {
  trackId: string;
  trackTitle: string;
  onBack: () => void;
}

export default function LeaderboardView({ trackId, trackTitle, onBack }: LeaderboardViewProps) {
  const [entries, setEntries] = useState<LeaderboardEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadLeaderboard = async () => {
      try {
        setLoading(true);
        const data = await invoke<LeaderboardEntry[]>('get_leaderboard', {
          trackId,
          limit: 20,
        });
        setEntries(data);
      } catch (e) {
        setError(String(e));
      } finally {
        setLoading(false);
      }
    };

    loadLeaderboard();
  }, [trackId]);

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleDateString();
  };

  return (
    <div style={styles.container}>
      <h1 style={styles.title}>Leaderboard</h1>
      <p style={styles.trackTitle}>{trackTitle}</p>

      {loading && <p style={styles.loading}>Loading...</p>}

      {error && <p style={styles.error}>{error}</p>}

      {!loading && !error && entries.length === 0 && (
        <p style={styles.noEntries}>
          No scores yet. Be the first to play!
        </p>
      )}

      {!loading && !error && entries.length > 0 && (
        <div style={styles.table}>
          <div style={styles.headerRow}>
            <span style={styles.rankCell}>Rank</span>
            <span style={styles.nameCell}>Player</span>
            <span style={styles.scoreCell}>Score</span>
            <span style={styles.dateCell}>Date</span>
          </div>
          {entries.map((entry, index) => (
            <div
              key={entry.id}
              style={{
                ...styles.row,
                ...(index === 0 ? styles.goldRow : {}),
                ...(index === 1 ? styles.silverRow : {}),
                ...(index === 2 ? styles.bronzeRow : {}),
              }}
            >
              <span style={styles.rankCell}>
                {index === 0 && '🥇'}
                {index === 1 && '🥈'}
                {index === 2 && '🥉'}
                {index > 2 && `#${index + 1}`}
              </span>
              <span style={styles.nameCell}>{entry.player_name}</span>
              <span style={styles.scoreCell}>{formatScore(entry.score)}</span>
              <span style={styles.dateCell}>{formatDate(entry.created_at)}</span>
            </div>
          ))}
        </div>
      )}

      <button onClick={onBack} style={styles.backButton}>
        Back
      </button>
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
  },
  title: {
    fontSize: 36,
    fontWeight: 'bold',
    marginBottom: 8,
    color: '#FFFFFF',
  },
  trackTitle: {
    fontSize: 20,
    color: '#88AAFF',
    marginBottom: 32,
  },
  loading: {
    color: '#888888',
  },
  error: {
    color: '#FF6666',
  },
  noEntries: {
    color: '#888888',
    textAlign: 'center',
    padding: 32,
  },
  table: {
    width: '100%',
    maxWidth: 600,
    marginBottom: 32,
  },
  headerRow: {
    display: 'flex',
    padding: '12px 16px',
    backgroundColor: '#1a1a2e',
    borderRadius: '8px 8px 0 0',
    fontWeight: 'bold',
    color: '#888888',
    fontSize: 14,
  },
  row: {
    display: 'flex',
    padding: '16px',
    backgroundColor: '#0f0f1a',
    borderBottom: '1px solid #222233',
    color: '#FFFFFF',
  },
  goldRow: {
    backgroundColor: 'rgba(255, 215, 0, 0.1)',
  },
  silverRow: {
    backgroundColor: 'rgba(192, 192, 192, 0.1)',
  },
  bronzeRow: {
    backgroundColor: 'rgba(205, 127, 50, 0.1)',
  },
  rankCell: {
    width: 60,
    textAlign: 'center',
  },
  nameCell: {
    flex: 1,
    textAlign: 'left',
  },
  scoreCell: {
    width: 100,
    textAlign: 'right',
    fontWeight: 'bold',
    color: '#FFD700',
  },
  dateCell: {
    width: 100,
    textAlign: 'right',
    color: '#888888',
    fontSize: 14,
  },
  backButton: {
    padding: '12px 32px',
    fontSize: 16,
    backgroundColor: 'transparent',
    color: '#888888',
    border: '1px solid #444444',
    borderRadius: 8,
    cursor: 'pointer',
  },
};
