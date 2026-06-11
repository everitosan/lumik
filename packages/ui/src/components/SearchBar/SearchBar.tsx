import type { InputHTMLAttributes, CSSProperties } from 'react';
import { Icon } from '../Icon';

export interface SearchBarProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  onSearch?: (value: string) => void;
}

const containerStyles: CSSProperties = {
  position: 'relative',
  display: 'flex',
  alignItems: 'center',
  width: '100%',
  maxWidth: '400px',
};

const inputStyles: CSSProperties = {
  width: '100%',
  padding: '10px 16px 10px 44px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  outline: 'none',
  transition: 'var(--lumik-transition-fast, 150ms ease)',
};

const iconStyles: CSSProperties = {
  position: 'absolute',
  left: '14px',
  pointerEvents: 'none',
};

export function SearchBar({
  placeholder = 'Buscar proyectos...',
  onSearch,
  onChange,
  style,
  ...props
}: SearchBarProps) {
  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    onChange?.(e);
    onSearch?.(e.target.value);
  };

  return (
    <div style={containerStyles}>
      <Icon
        name="search"
        size="lg"
        color="var(--lumik-on-surface-variant, #c2c6d7)"
        style={iconStyles}
      />
      <input
        type="search"
        placeholder={placeholder}
        onChange={handleChange}
        style={{ ...inputStyles, ...style }}
        {...props}
      />
    </div>
  );
}
