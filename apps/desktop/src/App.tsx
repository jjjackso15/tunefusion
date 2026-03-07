import { useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';

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
    () => peaks.map((peak, i) => ({
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
  const voicedFreqs = pitch_contour.frequencies_hz.filter(
    (f): f is number => f !== null
  );
  const meanFreq =
    voicedFreqs.length > 0
      ? voicedFreqs.reduce((a, b) => a + b, 0) / voicedFreqs.length
      : 0;

  return (
    <section>
      <p><strong>Run:</strong> {artifact.run_id}</p>
      <p><strong>Pipeline:</strong> {artifact.pipeline_version}</p>
      <p>
        <strong>Voiced frames:</strong> {voicedCount} / {totalFrames} (
        {totalFrames > 0 ? ((voicedCount / totalFrames) * 100).toFixed(1) : 0}%)
      </p>
      {meanFreq > 0 && (
        <p><strong>Mean pitch:</strong> {meanFreq.toFixed(1)} Hz</p>
      )}
    </section>
  );
}

export default function App() {
  const [selectedPath, setSelectedPath] = useState('');
  const [waveformArtifact, setWaveformArtifact] = useState<WaveformArtifact | null>(null);
  const [pitchArtifact, setPitchArtifact] = useState<PitchContourArtifact | null>(null);
  const [error, setError] = useState('');
  const [analyzing, setAnalyzing] = useState(false);

  const onSelectAudio = async () => {
    const isTauri = Boolean((window as any).__TAURI_INTERNALS__);

    if (!isTauri) {
      setError('File paths are not available in web mode. Run with: pnpm tauri dev');
      return;
    }

    const selected = await open({
      multiple: false,
      title: 'Select audio file',
      filters: [{ name: 'Audio', extensions: ['wav', 'mp3', 'flac', 'ogg'] }],
    });

    if (!selected) {
      setError('No file selected.');
      return;
    }

    const filePath = selected;
    setSelectedPath(filePath);
    setError('');
    setWaveformArtifact(null);
    setPitchArtifact(null);
    setAnalyzing(true);

    try {
      const [waveform, pitch] = await Promise.all([
        invoke<WaveformArtifact>('analyze_audio_file', { audioPath: filePath }),
        invoke<PitchContourArtifact>('analyze_pitch_contour', { audioPath: filePath }),
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

      <button onClick={onSelectAudio} disabled={analyzing} style={{ padding: '8px 16px', fontSize: 16 }}>
        {analyzing ? 'Analyzing...' : 'Open Audio File'}
      </button>

      {selectedPath && <p><strong>Selected:</strong> {selectedPath}</p>}

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
