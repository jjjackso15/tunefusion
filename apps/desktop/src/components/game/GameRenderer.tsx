import { useRef, useMemo } from 'react';
import { Canvas, useFrame } from '@react-three/fiber';
import * as THREE from 'three';
import { useGameStore, TargetPitch } from '../../stores/gameStore';
import { hzToLogY, getRatingColor } from '../../utils/pitchMath';

// Constants
const LANE_WIDTH = 10;
const LANE_HEIGHT = 6;
const NOTE_LOOK_AHEAD_MS = 3000; // Show notes 3 seconds ahead
const NOTE_LOOK_BEHIND_MS = 500; // Keep notes visible 0.5 seconds behind
const PLAYER_X_POSITION = -3; // Player line position

interface PitchLaneProps {
  minHz: number;
  maxHz: number;
}

/**
 * Single grid line component.
 */
function GridLine({ y }: { y: number }) {
  const line = useMemo(() => {
    const geometry = new THREE.BufferGeometry();
    const positions = new Float32Array([-LANE_WIDTH / 2, y, 0, LANE_WIDTH / 2, y, 0]);
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    const material = new THREE.LineBasicMaterial({ color: '#334455', opacity: 0.5, transparent: true });
    return new THREE.Line(geometry, material);
  }, [y]);

  return <primitive object={line} />;
}

/**
 * Grid lines for pitch reference.
 */
function PitchGrid({ minHz, maxHz }: PitchLaneProps) {
  const gridLines = useMemo(() => {
    // Draw horizontal lines at note positions (C notes)
    const noteFreqs = [65.41, 130.81, 261.63, 523.25]; // C2, C3, C4, C5
    return noteFreqs.map((freq) => (hzToLogY(freq, minHz, maxHz) - 0.5) * LANE_HEIGHT);
  }, [minHz, maxHz]);

  return (
    <>
      {gridLines.map((y, i) => (
        <GridLine key={i} y={y} />
      ))}
    </>
  );
}

/**
 * Target note visualization.
 */
function TargetNote({
  pitch,
  currentTimeMs,
  minHz,
  maxHz,
}: {
  pitch: TargetPitch;
  currentTimeMs: number;
  minHz: number;
  maxHz: number;
}) {
  const meshRef = useRef<THREE.Mesh>(null);

  // Only render voiced notes with valid frequency
  if (!pitch.voiced || pitch.frequency_hz === null) {
    return null;
  }

  // Calculate X position based on time
  const timeDiff = pitch.time_ms - currentTimeMs;
  const x = (timeDiff / NOTE_LOOK_AHEAD_MS) * (LANE_WIDTH / 2 - PLAYER_X_POSITION) + PLAYER_X_POSITION;

  // Check if note is visible
  if (timeDiff < -NOTE_LOOK_BEHIND_MS || timeDiff > NOTE_LOOK_AHEAD_MS) {
    return null;
  }

  // Calculate Y position
  const y = (hzToLogY(pitch.frequency_hz, minHz, maxHz) - 0.5) * LANE_HEIGHT;

  // Fade in/out at edges
  const alpha = Math.min(
    1,
    (NOTE_LOOK_AHEAD_MS - timeDiff) / 500,
    (timeDiff + NOTE_LOOK_BEHIND_MS) / 200
  );

  return (
    <mesh ref={meshRef} position={[x, y, 0]}>
      <boxGeometry args={[0.15, 0.1, 0.05]} />
      <meshStandardMaterial
        color="#4A90D9"
        emissive="#4A90D9"
        emissiveIntensity={0.5}
        transparent
        opacity={Math.max(0, Math.min(1, alpha))}
      />
    </mesh>
  );
}

/**
 * User pitch indicator (glowing trail).
 */
function UserPitchIndicator({ minHz, maxHz }: PitchLaneProps) {
  const meshRef = useRef<THREE.Mesh>(null);
  const trailRef = useRef<THREE.Points>(null);
  const { userPitch, userConfidence, streak } = useGameStore();

  // Trail history
  const trailHistory = useRef<{ y: number; opacity: number }[]>([]);

  useFrame(() => {
    if (!meshRef.current) return;

    if (userPitch !== null && userConfidence > 0.5) {
      const targetY = (hzToLogY(userPitch, minHz, maxHz) - 0.5) * LANE_HEIGHT;

      // Smooth movement
      meshRef.current.position.y += (targetY - meshRef.current.position.y) * 0.3;

      // Scale based on confidence
      const scale = 0.5 + userConfidence * 0.5;
      meshRef.current.scale.setScalar(scale);

      // Add to trail
      trailHistory.current.unshift({ y: meshRef.current.position.y, opacity: 1 });
      if (trailHistory.current.length > 30) {
        trailHistory.current.pop();
      }
    }

    // Fade trail
    trailHistory.current.forEach((point, i) => {
      point.opacity *= 0.9;
    });
    trailHistory.current = trailHistory.current.filter((p) => p.opacity > 0.1);
  });

  const streakIntensity = Math.min(1, streak / 10);
  const glowColor = streak > 10 ? '#FF8800' : '#FFD700';

  return (
    <>
      {/* Main indicator */}
      <mesh ref={meshRef} position={[PLAYER_X_POSITION, 0, 0.1]}>
        <sphereGeometry args={[0.15, 16, 16]} />
        <meshStandardMaterial
          color={glowColor}
          emissive={glowColor}
          emissiveIntensity={0.5 + streakIntensity * 0.5}
        />
      </mesh>

      {/* Player line */}
      <PlayerLine />
    </>
  );
}

/**
 * Player position line.
 */
function PlayerLine() {
  const line = useMemo(() => {
    const geometry = new THREE.BufferGeometry();
    const positions = new Float32Array([
      PLAYER_X_POSITION, -LANE_HEIGHT / 2, 0,
      PLAYER_X_POSITION, LANE_HEIGHT / 2, 0,
    ]);
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    const material = new THREE.LineBasicMaterial({ color: '#FFFFFF', opacity: 0.3, transparent: true });
    return new THREE.Line(geometry, material);
  }, []);

  return <primitive object={line} />;
}

/**
 * Score display overlay.
 */
function ScoreOverlay() {
  const { score, streak, accuracyPct, showHitEffect, hitEffectRating, userPitch, userConfidence, targetPitches, debugMessage, playbackPosition } = useGameStore();

  // Find the current target pitch (closest to playback position)
  const currentTarget = targetPitches.find(t =>
    t.voiced && t.frequency_hz !== null && Math.abs(t.time_ms - playbackPosition) < 100
  );

  return (
    <div
      style={{
        position: 'absolute',
        top: 16,
        right: 16,
        color: 'white',
        fontFamily: 'monospace',
        textAlign: 'right',
        textShadow: '2px 2px 4px rgba(0,0,0,0.8)',
      }}
    >
      <div style={{ fontSize: 48, fontWeight: 'bold' }}>{score.toLocaleString()}</div>
      <div style={{ fontSize: 24, color: streak > 5 ? '#FFD700' : '#AAAAAA' }}>
        {streak}x Streak
      </div>
      <div style={{ fontSize: 18, color: '#88AAFF' }}>
        {accuracyPct.toFixed(1)}% Accuracy
      </div>

      {/* Hit effect */}
      {showHitEffect && hitEffectRating && (
        <div
          style={{
            position: 'fixed',
            top: '50%',
            left: '50%',
            transform: 'translate(-50%, -50%)',
            fontSize: 72,
            fontWeight: 'bold',
            color: getRatingColor(hitEffectRating),
            textShadow: `0 0 20px ${getRatingColor(hitEffectRating)}`,
            animation: 'hitPop 0.5s ease-out',
          }}
        >
          {hitEffectRating.toUpperCase()}!
        </div>
      )}

      {/* Debug info */}
      <div
        style={{
          position: 'absolute',
          bottom: 32,
          right: 16,
          fontSize: 12,
          color: '#aaa',
          textAlign: 'right',
          backgroundColor: 'rgba(0,0,0,0.7)',
          padding: 8,
          borderRadius: 4,
        }}
      >
        <div>Your pitch: {userPitch ? `${userPitch.toFixed(1)} Hz` : 'No signal'}</div>
        <div>Target pitch: {currentTarget?.frequency_hz ? `${currentTarget.frequency_hz.toFixed(1)} Hz` : 'None'}</div>
        <div>Confidence: {(userConfidence * 100).toFixed(0)}%</div>
        <div style={{ color: '#666', marginTop: 4, fontSize: 10 }}>{debugMessage}</div>
      </div>

      <style>{`
        @keyframes hitPop {
          0% { transform: translate(-50%, -50%) scale(0.5); opacity: 0; }
          50% { transform: translate(-50%, -50%) scale(1.2); opacity: 1; }
          100% { transform: translate(-50%, -50%) scale(1); opacity: 0; }
        }
      `}</style>
    </div>
  );
}

/**
 * Main game scene.
 */
function GameScene() {
  const { targetPitches, playbackPosition } = useGameStore();
  const minHz = 65;
  const maxHz = 1047;

  return (
    <>
      {/* Lighting */}
      <ambientLight intensity={0.3} />
      <pointLight position={[0, 0, 5]} intensity={1} />

      {/* Background */}
      <mesh position={[0, 0, -1]}>
        <planeGeometry args={[LANE_WIDTH * 1.5, LANE_HEIGHT * 1.5]} />
        <meshStandardMaterial color="#111122" />
      </mesh>

      {/* Pitch grid */}
      <PitchGrid minHz={minHz} maxHz={maxHz} />

      {/* Target notes */}
      {targetPitches.map((pitch, i) => (
        <TargetNote
          key={i}
          pitch={pitch}
          currentTimeMs={playbackPosition}
          minHz={minHz}
          maxHz={maxHz}
        />
      ))}

      {/* User pitch indicator */}
      <UserPitchIndicator minHz={minHz} maxHz={maxHz} />
    </>
  );
}

/**
 * Main game renderer component.
 */
export default function GameRenderer() {
  return (
    <div style={{ width: '100%', height: '100%', position: 'relative' }}>
      <Canvas
        camera={{ position: [0, 0, 8], fov: 50 }}
        style={{ background: '#0a0a15' }}
      >
        <GameScene />
      </Canvas>
      <ScoreOverlay />
    </div>
  );
}
