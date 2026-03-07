import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

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
  const [tracks, setTracks] = useState<TrackRecord[]>([]);
  const [selectedTrackId, setSelectedTrackId] = useState('');
  const [waveformArtifact, setWaveformArtifact] = useState<WaveformArtifact | null>(null);
  const [pitchArtifact, setPitchArtifact] = useState<PitchContourArtifact | null>(null);
  const [error, setError] = useState('');
  const [importing, setImporting] = useState(false);
  const [analyzing, setAnalyzing] = useState(false);

  const selectedTrack = tracks.find((t) => t.id === selectedTrackId) ?? null;

  const refreshTracks = async () => {
    const loaded = await invoke<TrackRecord[]>('list_tracks');
    setTracks(loaded);
    if (loaded.length > 0 && !selectedTrackId) {
      setSelectedTrackId(loaded[0].id);
    }
  };

  useEffect(() => {
    refreshTracks().catch((e) => setError(String(e)));
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

    try {
      const track = await invoke<TrackRecord>('import_track', { audioPath: selected });
      await refreshTracks();
      setSelectedTrackId(track.id);
      setWaveformArtifact(null);
      setPitchArtifact(null);
    } catch (e) {
      setError(String(e));
    } finally {
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

    try {
      const [waveform, pitch] = await Promise.all([
        invoke<WaveformArtifact>('analyze_audio_file', { trackId: selectedTrackId }),
        invoke<PitchContourArtifact>('analyze_pitch_contour', { trackId: selectedTrackId }),
      ]);
      setWaveformArtifact(waveform);
      setPitchArtifact(pitch);
    } catch (e) {
      setError(String(e));
    } finally {
      setAnalyzing(false);
    }
  };

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
      </section>

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
