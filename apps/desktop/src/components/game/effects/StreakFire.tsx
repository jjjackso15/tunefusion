import { useRef, useMemo } from 'react';
import { useFrame } from '@react-three/fiber';
import * as THREE from 'three';

interface StreakFireProps {
  position: [number, number, number];
  streak: number;
  active: boolean;
}

/**
 * Fire effect that appears when player has a high streak.
 */
export default function StreakFire({ position, streak, active }: StreakFireProps) {
  const pointsRef = useRef<THREE.Points>(null);
  const count = 50;

  const { positions, velocities, lifetimes } = useMemo(() => {
    const positions = new Float32Array(count * 3);
    const velocities: THREE.Vector3[] = [];
    const lifetimes: number[] = [];

    for (let i = 0; i < count; i++) {
      positions[i * 3] = (Math.random() - 0.5) * 0.3;
      positions[i * 3 + 1] = Math.random() * 0.5;
      positions[i * 3 + 2] = (Math.random() - 0.5) * 0.1;

      velocities.push(new THREE.Vector3(
        (Math.random() - 0.5) * 0.5,
        1 + Math.random() * 2,
        (Math.random() - 0.5) * 0.3
      ));

      lifetimes.push(Math.random());
    }

    return { positions, velocities, lifetimes };
  }, []);

  useFrame(({ clock }) => {
    if (!active || !pointsRef.current || streak < 10) return;

    const time = clock.elapsedTime;
    const positionsArray = pointsRef.current.geometry.attributes.position.array as Float32Array;
    const intensity = Math.min(1, (streak - 10) / 20); // Max intensity at 30 streak

    for (let i = 0; i < count; i++) {
      // Reset particle if it's gone too high
      const lifetime = (time + lifetimes[i]) % 1;

      positionsArray[i * 3] = (Math.random() - 0.5) * 0.3 + velocities[i].x * lifetime * 0.2;
      positionsArray[i * 3 + 1] = lifetime * 2 * intensity;
      positionsArray[i * 3 + 2] = (Math.random() - 0.5) * 0.1;
    }

    pointsRef.current.geometry.attributes.position.needsUpdate = true;

    // Update color based on streak
    const material = pointsRef.current.material as THREE.PointsMaterial;
    const hue = 0.08 - intensity * 0.08; // Orange to red
    material.color.setHSL(hue, 1, 0.5);
    material.opacity = 0.5 + intensity * 0.5;
  });

  if (!active || streak < 10) return null;

  return (
    <points ref={pointsRef} position={position}>
      <bufferGeometry>
        <bufferAttribute
          attach="attributes-position"
          args={[positions, 3]}
        />
      </bufferGeometry>
      <pointsMaterial
        color="#FF8800"
        size={0.15}
        transparent
        opacity={0.7}
        sizeAttenuation
        blending={THREE.AdditiveBlending}
      />
    </points>
  );
}
