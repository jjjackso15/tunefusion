import { useEffect, useRef, useCallback } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useGameStore, PlaybackTick, PitchEvent, ScoreUpdate, TargetPitch } from '../stores/gameStore';

interface GameStateChange {
  state: string;
  countdown: number | null;
}

/**
 * Hook that manages the game loop and event listeners.
 */
export function useGameLoop() {
  const unlistenRefs = useRef<UnlistenFn[]>([]);
  const animationFrameRef = useRef<number>();
  const lastTimeRef = useRef<number>(0);

  const {
    updatePlayback,
    updateUserPitch,
    updateScore,
    setGameState,
    setCountdown,
    setTargetPitches,
    triggerHitEffect,
    clearHitEffect,
    lastRating,
  } = useGameStore();

  // Set up event listeners
  useEffect(() => {
    const setupListeners = async () => {
      // Clean up existing listeners
      for (const unlisten of unlistenRefs.current) {
        unlisten();
      }
      unlistenRefs.current = [];

      // Target pitches event
      const unlistenTargets = await listen<TargetPitch[]>('game:target_pitches', (event) => {
        console.log('Received target pitches:', event.payload.length);
        setTargetPitches(event.payload);
      });
      unlistenRefs.current.push(unlistenTargets);

      // Playback tick events
      const unlistenPlayback = await listen<PlaybackTick>('game:playback_tick', (event) => {
        updatePlayback(event.payload);
      });
      unlistenRefs.current.push(unlistenPlayback);

      // User pitch events
      const unlistenPitch = await listen<PitchEvent>('game:user_pitch', (event) => {
        updateUserPitch(event.payload);
      });
      unlistenRefs.current.push(unlistenPitch);

      // Score update events
      const unlistenScore = await listen<ScoreUpdate>('game:score_update', (event) => {
        updateScore(event.payload);

        // Trigger hit effect for non-miss ratings
        if (event.payload.last_rating && event.payload.last_rating !== 'miss') {
          triggerHitEffect(event.payload.last_rating);
          // Clear after animation
          setTimeout(clearHitEffect, 500);
        }
      });
      unlistenRefs.current.push(unlistenScore);

      // Game state change events
      const unlistenState = await listen<GameStateChange>('game:state_change', (event) => {
        const stateMap: Record<string, string> = {
          ready: 'ready',
          countdown: 'countdown',
          playing: 'playing',
          paused: 'paused',
          finished: 'finished',
        };
        const newState = stateMap[event.payload.state] || event.payload.state;
        setGameState(newState as any);
        setCountdown(event.payload.countdown);
      });
      unlistenRefs.current.push(unlistenState);
    };

    setupListeners();

    return () => {
      for (const unlisten of unlistenRefs.current) {
        unlisten();
      }
      if (animationFrameRef.current) {
        cancelAnimationFrame(animationFrameRef.current);
      }
    };
  }, [updatePlayback, updateUserPitch, updateScore, setGameState, setCountdown, setTargetPitches, triggerHitEffect, clearHitEffect]);

  return null;
}

/**
 * Hook for smooth animations at 60fps.
 */
export function useAnimationFrame(callback: (deltaTime: number) => void) {
  const callbackRef = useRef(callback);
  const lastTimeRef = useRef<number>(0);

  useEffect(() => {
    callbackRef.current = callback;
  }, [callback]);

  useEffect(() => {
    const animate = (time: number) => {
      const deltaTime = time - lastTimeRef.current;
      lastTimeRef.current = time;
      callbackRef.current(deltaTime);
      requestAnimationFrame(animate);
    };

    const frameId = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(frameId);
  }, []);
}

/**
 * Hook for interpolating a value smoothly.
 */
export function useSmoothValue(targetValue: number, smoothing: number = 0.1) {
  const valueRef = useRef(targetValue);

  const update = useCallback(() => {
    valueRef.current += (targetValue - valueRef.current) * smoothing;
    return valueRef.current;
  }, [targetValue, smoothing]);

  return { value: valueRef.current, update };
}
