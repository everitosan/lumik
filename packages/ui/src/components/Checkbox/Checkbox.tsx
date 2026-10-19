import type { CSSProperties } from 'react';

export interface CheckboxProps {
  label: string;
  /** Texto secundario bajo la etiqueta. */
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  className?: string;
  style?: CSSProperties;
}

const containerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  padding: '12px 0',
  cursor: 'pointer',
};

const boxStyles: CSSProperties = {
  width: '20px',
  height: '20px',
  margin: 0,
  flexShrink: 0,
  accentColor: 'var(--lumik-primary, #b0c6ff)',
  cursor: 'inherit',
};

const textStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '2px',
};

const labelStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  userSelect: 'none',
};

const descriptionStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '12px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  userSelect: 'none',
};

export function Checkbox({
  label,
  description,
  checked,
  onChange,
  disabled = false,
  className,
  style,
}: CheckboxProps) {
  return (
    <label
      className={className}
      style={{
        ...containerStyles,
        cursor: disabled ? 'not-allowed' : 'pointer',
        opacity: disabled ? 0.6 : 1,
        ...style,
      }}
    >
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        disabled={disabled}
        style={boxStyles}
      />
      <span style={textStyles}>
        <span style={labelStyles}>{label}</span>
        {description && <span style={descriptionStyles}>{description}</span>}
      </span>
    </label>
  );
}
