import type { ButtonHTMLAttributes, CSSProperties } from 'react';
import { Icon, type IconName } from '../Icon';

export interface SectionButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon?: IconName;
  label: string;
  active?: boolean;
}

const baseStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  width: '100%',
  padding: '12px 16px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  fontWeight: 500,
  textAlign: 'left',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  backgroundColor: 'transparent',
  border: 'none',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  cursor: 'pointer',
  transition: 'var(--lumik-transition-fast, 150ms ease)',
};

const activeStyles: CSSProperties = {
  color: 'var(--lumik-primary, #b0c6ff)',
  backgroundColor: 'rgba(176, 198, 255, 0.1)',
};

export function SectionButton({
  icon,
  label,
  active = false,
  style,
  ...props
}: SectionButtonProps) {
  const combinedStyles: CSSProperties = {
    ...baseStyles,
    ...(active ? activeStyles : {}),
    ...style,
  };

  return (
    <button style={combinedStyles} {...props}>
      {icon && <Icon name={icon} size="lg" />}
      <span>{label}</span>
    </button>
  );
}
