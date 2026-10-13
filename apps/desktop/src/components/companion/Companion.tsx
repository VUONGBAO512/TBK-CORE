import type { CompanionMood } from '../../types';
import './Companion.css';

interface CompanionProps {
  mood: CompanionMood;
  label: string;
}

export function Companion({ mood, label }: CompanionProps) {
  return (
    <div className={`companion-wrapper mood-${mood}`} aria-label={`PC Doctor companion: ${label}`}>
      <div className="companion-body">
        {/* Head */}
        <div className="c-head">
          {/* Eyes */}
          <div className="c-eyes">
            <div className="c-eye left">
              <div className="c-pupil" />
            </div>
            <div className="c-eye right">
              <div className="c-pupil" />
            </div>
          </div>

          {/* Mouth — changes by mood */}
          <div className="c-mouth">
            {mood === 'healthy' && <div className="mouth-smile" />}
            {mood === 'warning' && <div className="mouth-flat" />}
            {mood === 'critical' && <div className="mouth-frown" />}
            {mood === 'thinking' && <div className="mouth-open" />}
            {mood === 'offline' && <div className="mouth-flat offline" />}
            {mood === 'idle' && <div className="mouth-smile small" />}
          </div>

          {/* Antenna / indicator */}
          <div className="c-antenna">
            <div className="antenna-stem" />
            <div className="antenna-ball" />
          </div>
        </div>

        {/* Body / torso */}
        <div className="c-torso">
          <div className="c-cross">
            <div className="cross-h" />
            <div className="cross-v" />
          </div>
        </div>

        {/* Legs */}
        <div className="c-legs">
          <div className="c-leg left" />
          <div className="c-leg right" />
        </div>
      </div>

      <div className="companion-label" aria-live="polite">{label}</div>
    </div>
  );
}
