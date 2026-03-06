import { ChangeEvent, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';

type AnalysisArtifact = {
  run_id: string;
  pipeline_version: string;
  source_audio_path: string;
  sample_rate: number;
  waveform_peaks: number[];
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

export default function App() {
  const [selectedPath, setSelectedPath] = useState('');
  const [artifact, setArtifact] = useState<AnalysisArtifact | null>(null);
  const [error, setError] = useState('');

  const onSelectAudio = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    // In Tauri runtime, File has a local filesystem path.
    const filePath = (file as File & { path?: string }).path;
    if (!filePath) {
      setError('Could not resolve local file path.');
      return;
    }

    setSelectedPath(filePath);
    setError('');

    try {
      const analysis = await invoke<AnalysisArtifact>('analyze_audio_file', { audioPath: filePath });
      setArtifact(analysis);
    } catch (e) {
      setError(String(e));
      setArtifact(null);
    }
  };

  return (
    <main style={{ fontFamily: 'system-ui', padding: 16, maxWidth: 900 }}>
      <h1>TuneFusion – Waveform Slice</h1>

      <label>
        Select audio (.wav):{' '}
        <input type="file" accept="audio/wav" onChange={onSelectAudio} />
      </label>

      {selectedPath && <p><strong>Selected:</strong> {selectedPath}</p>}

      {artifact && (
        <section>
          <p><strong>Run:</strong> {artifact.run_id}</p>
          <p><strong>Pipeline:</strong> {artifact.pipeline_version}</p>
          <p>
            <strong>Artifact:</strong>{' '}
            artifacts/analysis_runs/{artifact.run_id}/analysis.json
          </p>
          <Waveform peaks={artifact.waveform_peaks} />
        </section>
      )}

      {error && <p style={{ color: 'crimson' }}>{error}</p>}
    </main>
  );
}
