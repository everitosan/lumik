import { useCallback, useEffect, useRef } from 'react';
import flatpickr from 'flatpickr';
import 'flatpickr/dist/flatpickr.min.css';
import '../styles/flatpickr.css';

interface DateFieldProps {
  label: string;
  /** Fecha en formato AAAA-MM-DD, o '' sin fecha. */
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
  error?: string;
}

const fieldStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
};

const labelStyles: React.CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '12px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  textTransform: 'uppercase' as const,
  letterSpacing: '0.05em',
};

const inputStyles: React.CSSProperties = {
  width: '100%',
  padding: '10px 12px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  fontFamily: 'var(--lumik-font-primary, Inter, sans-serif)',
  fontSize: '14px',
  outline: 'none',
  cursor: 'pointer',
  boxSizing: 'border-box' as const,
};

const errorStyles: React.CSSProperties = {
  fontSize: '12px',
  color: 'var(--lumik-error, #ffb4ab)',
};

/** Campo de fecha con flatpickr (AAAA-MM-DD), compartido por los modales de proyecto. */
export function DateField({ label, value, onChange, placeholder, disabled, error }: DateFieldProps) {
  const fpInstance = useRef<flatpickr.Instance | null>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  // Callback ref: fires exactly when the node mounts/unmounts — no timing issues.
  // appendTo: document.body escapes the modal's overflow:hidden.
  const inputRef = useCallback((node: HTMLInputElement | null) => {
    if (node) {
      fpInstance.current = flatpickr(node, {
        dateFormat: 'Y-m-d',
        disableMobile: true,
        appendTo: document.body,
        monthSelectorType: 'static',
        onChange: (_dates, dateStr) => onChangeRef.current(dateStr),
      }) as flatpickr.Instance;
    } else {
      fpInstance.current?.destroy();
      fpInstance.current = null;
    }
  }, []);

  // Refleja en el picker los cambios de valor que vengan de fuera (precarga).
  useEffect(() => {
    const fp = fpInstance.current;
    if (!fp) return;
    const current = fp.selectedDates[0] ? fp.formatDate(fp.selectedDates[0], 'Y-m-d') : '';
    if (current !== value) {
      if (value) fp.setDate(value, false);
      else fp.clear(false);
    }
  }, [value]);

  return (
    <div style={fieldStyles}>
      <span style={labelStyles}>{label}</span>
      <input
        ref={inputRef}
        type="text"
        placeholder={placeholder}
        readOnly
        disabled={disabled}
        style={{
          ...inputStyles,
          border: error ? '1px solid var(--lumik-error, #ffb4ab)' : inputStyles.border,
          opacity: disabled ? 0.5 : 1,
          cursor: disabled ? 'not-allowed' : 'pointer',
        }}
      />
      {error && <span style={errorStyles}>{error}</span>}
    </div>
  );
}
