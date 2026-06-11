import type { ButtonHTMLAttributes, ReactNode, CSSProperties } from 'react';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
  size?: 'sm' | 'md' | 'lg';
  children: ReactNode;
  leftIcon?: ReactNode;
  rightIcon?: ReactNode;
  fullWidth?: boolean;
}

const baseStyles: CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  gap: '8px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontWeight: 500,
  border: '1px solid transparent',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  cursor: 'pointer',
  transition: 'var(--lumik-transition-fast, 150ms ease)',
  outline: 'none',
};

const sizeStyles: Record<string, CSSProperties> = {
  sm: { padding: '6px 12px', fontSize: '12px' },
  md: { padding: '10px 16px', fontSize: '14px' },
  lg: { padding: '14px 24px', fontSize: '16px' },
};

const variantStyles: Record<string, CSSProperties> = {
  primary: {
    backgroundColor: 'var(--lumik-primary-container, #558dff)',
    color: 'var(--lumik-on-primary, #002d6e)',
    borderColor: 'transparent',
  },
  secondary: {
    backgroundColor: 'transparent',
    color: 'var(--lumik-on-surface, #e5e2e1)',
    borderColor: 'var(--lumik-outline-variant, #424654)',
  },
  ghost: {
    backgroundColor: 'transparent',
    color: 'var(--lumik-on-surface-variant, #c2c6d7)',
    borderColor: 'transparent',
  },
  danger: {
    backgroundColor: 'var(--lumik-error-container, #93000a)',
    color: 'var(--lumik-error, #ffb4ab)',
    borderColor: 'transparent',
  },
};

export function Button({
  variant = 'primary',
  size = 'md',
  children,
  leftIcon,
  rightIcon,
  fullWidth = false,
  disabled,
  style,
  ...props
}: ButtonProps) {
  const combinedStyles: CSSProperties = {
    ...baseStyles,
    ...sizeStyles[size],
    ...variantStyles[variant],
    ...(fullWidth && { width: '100%' }),
    ...(disabled && { opacity: 0.5, cursor: 'not-allowed' }),
    ...style,
  };

  return (
    <button style={combinedStyles} disabled={disabled} {...props}>
      {leftIcon}
      {children}
      {rightIcon}
    </button>
  );
}
