import { GameMode } from '../../stores/gameStore';

interface ModeSelectorProps {
  onSelect: (mode: GameMode) => void;
  onBack: () => void;
}

export default function ModeSelector({ onSelect, onBack }: ModeSelectorProps) {
  return (
    <div style={styles.container}>
      <h1 style={styles.title}>Game Mode</h1>

      <div style={styles.modeGrid}>
        <button onClick={() => onSelect('solo')} style={styles.modeButton}>
          <div style={styles.modeIcon}>🎤</div>
          <div style={styles.modeName}>Solo Practice</div>
          <div style={styles.modeDescription}>
            Practice singing on your own. Perfect your pitch and timing without pressure.
          </div>
        </button>

        <button onClick={() => onSelect('competition')} style={styles.modeButton}>
          <div style={styles.modeIcon}>⚔️</div>
          <div style={styles.modeName}>Competition</div>
          <div style={styles.modeDescription}>
            Take turns with a friend and compete for the highest score!
          </div>
        </button>
      </div>

      <button onClick={onBack} style={styles.backButton}>
        Back to Main
      </button>
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
  },
  title: {
    fontSize: 48,
    fontWeight: 'bold',
    marginBottom: 48,
    color: '#FFFFFF',
  },
  modeGrid: {
    display: 'flex',
    gap: 32,
    marginBottom: 48,
  },
  modeButton: {
    width: 280,
    padding: 32,
    backgroundColor: '#1a1a2e',
    border: '2px solid #333366',
    borderRadius: 16,
    cursor: 'pointer',
    transition: 'all 0.2s ease',
    textAlign: 'center',
  },
  modeIcon: {
    fontSize: 64,
    marginBottom: 16,
  },
  modeName: {
    fontSize: 24,
    fontWeight: 'bold',
    color: '#FFFFFF',
    marginBottom: 12,
  },
  modeDescription: {
    fontSize: 14,
    color: '#888888',
    lineHeight: 1.5,
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
