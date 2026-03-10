import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

interface ParticleBurstProps {
  position: [number, number, number];
  color: string;
  count?: number;
  active: boolean;
  onComplete?: () => void;
}

/**
 * Particle burst effect for Perfect/Great hits.
 */
export default function ParticleBurst({
  position,
  color,
  count = 20,
  active,
  onComplete,
}: ParticleBurstProps) {
  const pointsRef = useRef<THREE.Points>(null);
  const startTimeRef = useRef<number>(0);
  const hasCompleted = useRef(false);

  const { positions, velocities } = useMemo(() => {
    const positions = new Float32Array(count * 3);
    const velocities: THREE.Vector3[] = [];

    for (let i = 0; i < count; i++) {
      positions[i * 3] = 0;
      positions[i * 3 + 1] = 0;
      positions[i * 3 + 2] = 0;

      // Random velocity outward
      const angle = Math.random() * Math.PI * 2;
      const speed = 0.5 + Math.random() * 1.5;
      velocities.push(new THREE.Vector3(
        Math.cos(angle) * speed,
        Math.sin(angle) * speed,
        (Math.random() - 0.5) * 0.5
      ));
    }

    return { positions, velocities };
  }, [count]);

  useFrame(({ clock }) => {
    if (!active || !pointsRef.current) return;

    if (startTimeRef.current === 0) {
      startTimeRef.current = clock.elapsedTime;
      hasCompleted.current = false;
    }

    const elapsed = clock.elapsedTime - startTimeRef.current;
    const duration = 0.8;

    if (elapsed > duration) {
      if (!hasCompleted.current) {
        hasCompleted.current = true;
        startTimeRef.current = 0;
        onComplete?.();
      }
      return;
    }

    const progress = elapsed / duration;
    const positionsArray = pointsRef.current.geometry.attributes.position.array as Float32Array;

    for (let i = 0; i < count; i++) {
      const vel = velocities[i];
      const decay = 1 - progress;

      positionsArray[i * 3] = vel.x * elapsed * 2 * decay;
      positionsArray[i * 3 + 1] = vel.y * elapsed * 2 * decay - elapsed * elapsed * 2; // gravity
      positionsArray[i * 3 + 2] = vel.z * elapsed * decay;
    }

    pointsRef.current.geometry.attributes.position.needsUpdate = true;

    // Fade out
    const material = pointsRef.current.material as THREE.PointsMaterial;
    material.opacity = 1 - progress;
  });

  if (!active) return null;

  return (
    <points ref={pointsRef} position={position}>
      <bufferGeometry>
        <bufferAttribute
          attach="attributes-position"
          args={[positions, 3]}
        />
      </bufferGeometry>
      <pointsMaterial
        color={color}
        size={0.1}
        transparent
        opacity={1}
        sizeAttenuation
      />
    </points>
  );
}
