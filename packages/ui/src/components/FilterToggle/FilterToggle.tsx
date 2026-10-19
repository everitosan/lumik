import type { CSSProperties, ReactNode } from 'react';

export interface FilterToggleProps {
  /** Texto del filtro; puede incluir un contador. */
  label: ReactNode;
  checked: boolean;
  onChange: (checked: boolean) => void;
  className?: string;
  style?: CSSProperties;
}

const buttonStyles = (checked: boolean): CSSProperties => ({
  display: 'flex',
  alignItems: 'center',
  gap: '7px',
  padding: 0,
  background: 'transparent',
  border: 'none',
  cursor: 'pointer',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  color: checked ? 'var(--lumik-tertiary, #ffb690)' : 'var(--lumik-on-surface-variant, #c2c6d7)',
  whiteSpace: 'nowrap',
  userSelect: 'none',
});

const circleStyles = (checked: boolean): CSSProperties => ({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: '16px',
  height: '16px',
  boxSizing: 'border-box',
  borderRadius: '50%',
  border: `2px solid ${checked ? 'var(--lumik-tertiary, #ffb690)' : 'var(--lumik-outline-variant, #424654)'}`,
  backgroundColor: checked ? 'var(--lumik-tertiary, #ffb690)' : 'transparent',
  flexShrink: 0,
  transition: 'background-color 0.15s, border-color 0.15s',
});

/** Interruptor redondo de la barra de filtros (filtros de sí/no). */
export function FilterToggle({ label, checked, onChange, className, style }: FilterToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      onClick={() => onChange(!checked)}
      className={className}
      style={{ ...buttonStyles(checked), ...style }}
    >
      <span style={circleStyles(checked)} aria-hidden="true">
        {checked && (
          <svg width="8" height="8" viewBox="0 0 8 8" fill="none">
            <path
              d="M1 4l2 2 4-4"
              stroke="var(--lumik-on-tertiary, #2d1600)"
              strokeWidth="1.5"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        )}
      </span>
      {label}
    </button>
  );
}
