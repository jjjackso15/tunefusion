import { useEffect, useMemo, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';
import { GameScreen } from './components/game';
import { useGameStore, TrackInfo } from './stores/gameStore';

type AnalysisProgress = {
  phase: string;
  step: number;
  total_steps: number;
  message: string;
};

type AnalysisResult = {
  waveform: WaveformArtifact;
  pitch: PitchContourArtifact;
};

function Spinner() {
  return (
    <span
      style={{
        display: 'inline-block',
        width: 16,
        height: 16,
        border: '2px solid #ccc',
        borderTopColor: '#333',
        borderRadius: '50%',
        animation: 'spin 1s linear infinite',
        marginRight: 8,
        verticalAlign: 'middle',
      }}
    />
  );
}

function ProgressIndicator({
  title,
  description,
  elapsedSeconds,
  step,
  totalSteps,
}: {
  title: string;
  description: string;
  elapsedSeconds: number;
  step?: number;
  totalSteps?: number;
}) {
  const progressPercent = step && totalSteps ? (step / totalSteps) * 100 : 0;

  return (
    <div
      style={{
        padding: 16,
        marginBottom: 16,
        backgroundColor: '#f0f4ff',
        border: '1px solid #c0d0ff',
        borderRadius: 8,
      }}
    >
      <div style={{ display: 'flex', alignItems: 'center', marginBottom: 8 }}>
        <Spinner />
        <strong>{title}</strong>
      </div>
      <p style={{ margin: 0, fontSize: 14, color: '#666' }}>{description}</p>

      {step !== undefined && totalSteps !== undefined && (
        <div style={{ marginTop: 12 }}>
          <div
            style={{
              height: 8,
              backgroundColor: '#ddd',
              borderRadius: 4,
              overflow: 'hidden',
            }}
          >
            <div
              style={{
                height: '100%',
                width: `${progressPercent}%`,
                backgroundColor: '#4a90d9',
                borderRadius: 4,
                transition: 'width 0.3s ease',
              }}
            />
          </div>
          <p style={{ margin: '8px 0 0', fontSize: 12, color: '#888' }}>
            Step {step} of {totalSteps}
          </p>
        </div>
      )}

      <p style={{ margin: '8px 0 0', fontSize: 14, color: '#888' }}>
        Elapsed: {elapsedSeconds}s
      </p>
      <style>{`
        @keyframes spin {
          to { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );
}

type TrackRecord = {
  id: string;
  title: string;
  audio_path: string;
  audio_hash: string;
  sample_rate: number;
  duration_seconds: number;
  created_at: string;
};

type WaveformArtifact = {
  run_id: string;
  pipeline_version: string;
  sample_rate: number;
  waveform_peaks: number[];
};

type PitchContourArtifact = {
  run_id: string;
  pipeline_version: string;
  sample_rate: number;
  pitch_contour: {
    times_s: number[];
    frequencies_hz: (number | null)[];
    voiced: boolean[];
    voiced_prob: number[];
  };
};

function Waveform({ peaks }: { peaks: number[] }) {
  const bars = useMemo(
    () =>
      peaks.map((peak, i) => ({
        x: i,
        h: Math.max(1, Math.round(peak * 100)),
      })),
    [peaks]
  );

  return (
    <svg width="100%" height="140" viewBox={`0 0 ${Math.max(1, bars.length)} 100`} preserveAspectRatio="none">
      {bars.map((bar) => (
        <line
          key={bar.x}
          x1={bar.x}
          x2={bar.x}
          y1={50 - bar.h / 2}
          y2={50 + bar.h / 2}
          stroke="currentColor"
          strokeWidth="0.8"
        />
      ))}
    </svg>
  );
}

function PitchStats({ artifact }: { artifact: PitchContourArtifact }) {
  const { pitch_contour } = artifact;
  const voicedCount = pitch_contour.voiced.filter(Boolean).length;
  const totalFrames = pitch_contour.voiced.length;
  const voicedFreqs = pitch_contour.frequencies_hz.filter((f): f is number => f !== null);
  const meanFreq = voicedFreqs.length > 0 ? voicedFreqs.reduce((a, b) => a + b, 0) / voicedFreqs.length : 0;

  return (
    <section>
      <p><strong>Run:</strong> {artifact.run_id}</p>
      <p><strong>Pipeline:</strong> {artifact.pipeline_version}</p>
      <p>
        <strong>Voiced frames:</strong> {voicedCount} / {totalFrames} (
        {totalFrames > 0 ? ((voicedCount / totalFrames) * 100).toFixed(1) : 0}%)
      </p>
      {meanFreq > 0 && <p><strong>Mean pitch:</strong> {meanFreq.toFixed(1)} Hz</p>}
    </section>
  );
}

export default function App() {
  console.log('TuneFusion: App component rendering');

  const [tracks, setTracks] = useState<TrackRecord[]>([]);
  const [selectedTrackId, setSelectedTrackId] = useState('');
  const [waveformArtifact, setWaveformArtifact] = useState<WaveformArtifact | null>(null);
  const [pitchArtifact, setPitchArtifact] = useState<PitchContourArtifact | null>(null);
  const [error, setError] = useState('');
  const [importing, setImporting] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);
  const [elapsedSeconds, setElapsedSeconds] = useState(0);
  const [analysisProgress, setAnalysisProgress] = useState<AnalysisProgress | null>(null);
  const [showGameMode, setShowGameMode] = useState(false);
  const [appReady, setAppReady] = useState(false);
  const timerRef = useRef<number | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const { setGameState } = useGameStore();
  const selectedTrack = tracks.find((t) => t.id === selectedTrackId) ?? null;

  const refreshTracks = async () => {
    const loaded = await invoke<TrackRecord[]>('list_tracks');
    setTracks(loaded);
    if (loaded.length > 0 && !selectedTrackId) {
      setSelectedTrackId(loaded[0].id);
    }
  };

  useEffect(() => {
    console.log('TuneFusion: initializing, loading tracks...');
    refreshTracks()
      .then(() => {
        console.log('TuneFusion: tracks loaded successfully');
        setAppReady(true);
      })
      .catch((e) => {
        console.error('TuneFusion: failed to load tracks:', e);
        setError(String(e));
        setAppReady(true); // Still mark as ready to show error
      });
  }, []);

  const onImportAudio = async () => {
    const isTauri = Boolean((window as any).__TAURI_INTERNALS__);
    if (!isTauri) {
      setError('File paths are not available in web mode. Run with: pnpm tauri dev');
      return;
    }

    const selected = await open({
      multiple: false,
      title: 'Import audio file',
      filters: [{ name: 'Audio', extensions: ['wav', 'mp3', 'flac', 'ogg'] }],
    });

    if (!selected) {
      return;
    }

    setError('');
    setImporting(true);
    setElapsedSeconds(0);

    // Start elapsed time counter
    const startTime = Date.now();
    timerRef.current = window.setInterval(() => {
      setElapsedSeconds(Math.floor((Date.now() - startTime) / 1000));
    }, 1000);

    try {
      const track = await invoke<TrackRecord>('import_track', { audioPath: selected });
      await refreshTracks();
      setSelectedTrackId(track.id);
      setWaveformArtifact(null);
      setPitchArtifact(null);
    } catch (e) {
      setError(String(e));
    } finally {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
      setImporting(false);
    }
  };

  const onAnalyzeTrack = async () => {
    if (!selectedTrackId) {
      setError('Select a track first.');
      return;
    }

    setError('');
    setAnalyzing(true);
    setWaveformArtifact(null);
    setPitchArtifact(null);
    setElapsedSeconds(0);
    setAnalysisProgress(null);

    // Start elapsed time counter
    const startTime = Date.now();
    timerRef.current = window.setInterval(() => {
      setElapsedSeconds(Math.floor((Date.now() - startTime) / 1000));
    }, 1000);

    // Listen for progress events
    unlistenRef.current = await listen<AnalysisProgress>('analysis-progress', (event) => {
      setAnalysisProgress(event.payload);
    });

    try {
      const result = await invoke<AnalysisResult>('analyze_track', { trackId: selectedTrackId });
      setWaveformArtifact(result.waveform);
      setPitchArtifact(result.pitch);
    } catch (e) {
      setError(String(e));
    } finally {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
      setAnalyzing(false);
      setAnalysisProgress(null);
    }
  };

  // Convert tracks to game-compatible format
  const gameTracks: TrackInfo[] = tracks.map((t) => ({
    id: t.id,
    title: t.title,
    duration_seconds: t.duration_seconds,
  }));

  // Handle entering game mode
  const onStartGame = () => {
    setShowGameMode(true);
    setGameState('mode_select');
  };

  // Handle exiting game mode
  const onExitGame = () => {
    setShowGameMode(false);
    setGameState('idle');
  };

  // If in game mode, render the game screen
  if (showGameMode) {
    return <GameScreen tracks={gameTracks} onBack={onExitGame} />;
  }

  // Show loading state while initializing
  if (!appReady) {
    return (
      <main style={{ fontFamily: 'system-ui', padding: 32, textAlign: 'center' }}>
        <h1>TuneFusion</h1>
        <p style={{ color: '#666', marginTop: 16 }}>Loading...</p>
      </main>
    );
  }

  return (
    <main style={{ fontFamily: 'system-ui', padding: 16, maxWidth: 900 }}>
      <h1>TuneFusion</h1>

      <section style={{ display: 'flex', gap: 12, marginBottom: 16 }}>
        <button onClick={onImportAudio} disabled={importing || analyzing} style={{ padding: '8px 16px', fontSize: 16 }}>
          {importing ? 'Importing...' : 'Import Audio File'}
        </button>

        <button
          onClick={onAnalyzeTrack}
          disabled={analyzing || importing || !selectedTrackId}
          style={{ padding: '8px 16px', fontSize: 16 }}
        >
          {analyzing ? 'Analyzing...' : 'Analyze Selected Track'}
        </button>

        <button
          onClick={onStartGame}
          disabled={importing || analyzing || tracks.length === 0}
          style={{
            padding: '8px 16px',
            fontSize: 16,
            backgroundColor: '#4A90D9',
            color: 'white',
            border: 'none',
            borderRadius: 4,
            cursor: tracks.length === 0 ? 'not-allowed' : 'pointer',
          }}
        >
          Play Game
        </button>
      </section>

      {importing && (
        <ProgressIndicator
          title="Importing audio..."
          description="Decoding audio file and computing hash. This may take a moment for larger files."
          elapsedSeconds={elapsedSeconds}
        />
      )}

      {analyzing && (
        <ProgressIndicator
          title={analysisProgress?.message || "Analyzing audio..."}
          description={
            analysisProgress?.phase === 'pitch'
              ? "Pitch detection is CPU-intensive. Use --release flag for faster analysis."
              : "Processing your audio file..."
          }
          elapsedSeconds={elapsedSeconds}
          step={analysisProgress?.step}
          totalSteps={analysisProgress?.total_steps}
        />
      )}

      <section>
        <h2>Tracks</h2>
        {tracks.length === 0 && <p>No tracks imported yet.</p>}
        {tracks.length > 0 && (
          <select
            value={selectedTrackId}
            onChange={(e) => setSelectedTrackId(e.target.value)}
            style={{ width: '100%', maxWidth: 720, padding: 8, marginBottom: 8 }}
          >
            {tracks.map((track) => (
              <option key={track.id} value={track.id}>
                {track.title} ({track.duration_seconds.toFixed(1)}s)
              </option>
            ))}
          </select>
        )}
        {selectedTrack && <p><strong>Path:</strong> {selectedTrack.audio_path}</p>}
      </section>

      {waveformArtifact && (
        <section>
          <h2>Waveform</h2>
          <p><strong>Run:</strong> {waveformArtifact.run_id}</p>
          <Waveform peaks={waveformArtifact.waveform_peaks} />
        </section>
      )}

      {pitchArtifact && (
        <section>
          <h2>Pitch Contour</h2>
          <PitchStats artifact={pitchArtifact} />
        </section>
      )}

      {error && <p style={{ color: 'crimson' }}>{error}</p>}
    </main>
  );
}
