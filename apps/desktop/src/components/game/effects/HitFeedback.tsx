import { useState, useEffect } from 'react';
import { HitRating } from '../../../stores/gameStore';
import { getRatingColor } from '../../../utils/pitchMath';

interface HitFeedbackProps {
  rating: HitRating | null;
  show: boolean;
}

/**
 * Hit feedback popup that shows rating text.
 */
export default function HitFeedback({ rating, show }: HitFeedbackProps) {
  const [visible, setVisible] = useState(false);
  const [animating, setAnimating] = useState(false);

  useEffect(() => {
    if (show && rating) {
      setVisible(true);
      setAnimating(true);

      const timer = setTimeout(() => {
        setAnimating(false);
        setTimeout(() => setVisible(false), 100);
      }, 500);

      return () => clearTimeout(timer);
    }
  }, [show, rating]);

  if (!visible || !rating) return null;

  const color = getRatingColor(rating);
  const text = rating.toUpperCase();

  return (
    <div
      style={{
        position: 'fixed',
        top: '40%',
        left: '50%',
        transform: 'translate(-50%, -50%)',
        fontSize: 72,
        fontWeight: 'bold',
        fontFamily: 'system-ui, sans-serif',
        color: color,
        textShadow: `0 0 30px ${color}, 0 0 60px ${color}`,
        opacity: animating ? 1 : 0,
        transition: 'opacity 0.1s ease-out',
        pointerEvents: 'none',
        zIndex: 1000,
        animation: animating ? 'hitPop 0.5s ease-out' : 'none',
      }}
    >
      {text}!
      <style>{`
        @keyframes hitPop {
          0% {
            transform: translate(-50%, -50%) scale(0.5);
            opacity: 0;
          }
          30% {
            transform: translate(-50%, -50%) scale(1.3);
            opacity: 1;
          }
          100% {
            transform: translate(-50%, -50%) scale(1);
            opacity: 0;
          }
        }
      `}</style>
    </div>
  );
}
