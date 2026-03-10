import { useState } from 'react';
import { TrackInfo } from '../../stores/gameStore';
import { formatTime } from '../../utils/pitchMath';

interface PlayerSelectorProps {
  tracks: TrackInfo[];
  selectedTrack: TrackInfo | null;
  onTrackSelect: (track: TrackInfo) => void;
  onPlayerSubmit: (name: string) => void;
  onBack: () => void;
}

export default function PlayerSelector({
  tracks,
  selectedTrack,
  onTrackSelect,
  onPlayerSubmit,
  onBack,
}: PlayerSelectorProps) {
  const [playerName, setPlayerName] = useState('');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (playerName.trim() && selectedTrack) {
      onPlayerSubmit(playerName.trim());
    }
  };

  return (
    <div style={styles.container}>
      <h1 style={styles.title}>Select Track & Player</h1>

      {/* Track selection */}
      <div style={styles.section}>
        <h2 style={styles.sectionTitle}>Choose a Track</h2>
        {tracks.length === 0 ? (
          <p style={styles.noTracks}>
            No tracks available. Import and analyze a track first!
          </p>
        ) : (
          <div style={styles.trackList}>
            {tracks.map((track) => (
              <button
                key={track.id}
                onClick={() => onTrackSelect(track)}
                style={{
                  ...styles.trackButton,
                  ...(selectedTrack?.id === track.id ? styles.trackButtonSelected : {}),
                }}
              >
                <div style={styles.trackTitle}>{track.title}</div>
                <div style={styles.trackDuration}>
                  {formatTime(track.duration_seconds * 1000)}
                </div>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Player name input */}
      <div style={styles.section}>
        <h2 style={styles.sectionTitle}>Enter Your Name</h2>
        <form onSubmit={handleSubmit} style={styles.form}>
          <input
            type="text"
            value={playerName}
            onChange={(e) => setPlayerName(e.target.value)}
            placeholder="Player name..."
            style={styles.input}
            maxLength={20}
          />
          <button
            type="submit"
            disabled={!playerName.trim() || !selectedTrack}
            style={{
              ...styles.submitButton,
              ...(!playerName.trim() || !selectedTrack ? styles.submitButtonDisabled : {}),
            }}
          >
            Continue
          </button>
        </form>
      </div>

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
    overflowY: 'auto',
  },
  title: {
    fontSize: 36,
    fontWeight: 'bold',
    marginBottom: 32,
    color: '#FFFFFF',
  },
  section: {
    width: '100%',
    maxWidth: 600,
    marginBottom: 32,
  },
  sectionTitle: {
    fontSize: 20,
    color: '#88AAFF',
    marginBottom: 16,
  },
  noTracks: {
    color: '#888888',
    textAlign: 'center',
    padding: 32,
    border: '1px dashed #444444',
    borderRadius: 8,
  },
  trackList: {
    display: 'flex',
    flexDirection: 'column',
    gap: 8,
    maxHeight: 300,
    overflowY: 'auto',
  },
  trackButton: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '16px 20px',
    backgroundColor: '#1a1a2e',
    border: '1px solid #333366',
    borderRadius: 8,
    cursor: 'pointer',
    transition: 'all 0.2s ease',
  },
  trackButtonSelected: {
    backgroundColor: '#2a2a4e',
    borderColor: '#4A90D9',
  },
  trackTitle: {
    fontSize: 16,
    color: '#FFFFFF',
    textAlign: 'left',
  },
  trackDuration: {
    fontSize: 14,
    color: '#888888',
  },
  form: {
    display: 'flex',
    gap: 12,
  },
  input: {
    flex: 1,
    padding: '14px 18px',
    fontSize: 18,
    backgroundColor: '#1a1a2e',
    border: '1px solid #333366',
    borderRadius: 8,
    color: '#FFFFFF',
    outline: 'none',
  },
  submitButton: {
    padding: '14px 28px',
    fontSize: 18,
    fontWeight: 'bold',
    backgroundColor: '#4A90D9',
    color: 'white',
    border: 'none',
    borderRadius: 8,
    cursor: 'pointer',
    transition: 'all 0.2s ease',
  },
  submitButtonDisabled: {
    backgroundColor: '#333366',
    cursor: 'not-allowed',
  },
  backButton: {
    padding: '12px 32px',
    fontSize: 16,
    backgroundColor: 'transparent',
    color: '#888888',
    border: '1px solid #444444',
    borderRadius: 8,
    cursor: 'pointer',
    marginTop: 16,
  },
};
