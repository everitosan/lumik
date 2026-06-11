import type { CSSProperties, InputHTMLAttributes, TextareaHTMLAttributes } from 'react';

export type InputVariant = 'text' | 'date' | 'textarea';

interface BaseInputProps {
  label?: string;
  error?: string;
  variant?: InputVariant;
  fullWidth?: boolean;
  className?: string;
  style?: CSSProperties;
}

export type InputProps = BaseInputProps &
  (
    | ({ variant?: 'text' | 'date' } & Omit<InputHTMLAttributes<HTMLInputElement>, 'style'>)
    | ({ variant: 'textarea' } & Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, 'style'>)
  );

const containerStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
};

const labelStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const baseInputStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  backgroundColor: 'var(--lumik-surface-container-lowest, #0e0e0e)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  padding: '10px 12px',
  outline: 'none',
  transition: 'border-color var(--lumik-transition-fast, 150ms ease), box-shadow var(--lumik-transition-fast, 150ms ease)',
};

const textareaStyles: CSSProperties = {
  ...baseInputStyles,
  minHeight: '100px',
  resize: 'vertical',
};

const errorInputStyles: CSSProperties = {
  borderColor: 'var(--lumik-error, #ffb4ab)',
};

const errorTextStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '12px',
  color: 'var(--lumik-error, #ffb4ab)',
};

export function Input({
  label,
  error,
  variant = 'text',
  fullWidth = false,
  className,
  style,
  ...props
}: InputProps) {
  const containerStyle: CSSProperties = {
    ...containerStyles,
    width: fullWidth ? '100%' : 'auto',
  };

  const inputStyle: CSSProperties = {
    ...(variant === 'textarea' ? textareaStyles : baseInputStyles),
    ...(error ? errorInputStyles : {}),
    width: fullWidth ? '100%' : 'auto',
    boxSizing: 'border-box',
  };

  return (
    <div style={{ ...containerStyle, ...style }} className={className}>
      {label && <label style={labelStyles}>{label}</label>}

      {variant === 'textarea' ? (
        <textarea
          style={inputStyle}
          {...(props as TextareaHTMLAttributes<HTMLTextAreaElement>)}
        />
      ) : (
        <input
          type={variant === 'date' ? 'date' : 'text'}
          style={inputStyle}
          {...(props as InputHTMLAttributes<HTMLInputElement>)}
        />
      )}

      {error && <span style={errorTextStyles}>{error}</span>}
    </div>
  );
}
