import { useState } from 'react';
import type { InputHTMLAttributes, CSSProperties, Ref } from 'react';
import { Icon } from '../Icon';

export interface SearchBarProps extends Omit<InputHTMLAttributes<HTMLInputElement>, 'type'> {
  onSearch?: (value: string) => void;
  ref?: Ref<HTMLInputElement>;
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
  onFocus,
  onBlur,
  ref,
  style,
  ...props
}: SearchBarProps) {
  const [focused, setFocused] = useState(false);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    onChange?.(e);
    onSearch?.(e.target.value);
  };

  const focusStyles: CSSProperties = focused
    ? { borderColor: 'var(--lumik-primary, #aac0f0)', boxShadow: '0 0 0 2px color-mix(in srgb, var(--lumik-primary, #aac0f0) 25%, transparent)' }
    : {};

  return (
    <div style={containerStyles}>
      <Icon
        name="search"
        size="lg"
        color="var(--lumik-on-surface-variant, #c2c6d7)"
        style={iconStyles}
      />
      <input
        ref={ref}
        type="search"
        placeholder={placeholder}
        onChange={handleChange}
        onFocus={(e) => { setFocused(true); onFocus?.(e); }}
        onBlur={(e) => { setFocused(false); onBlur?.(e); }}
        style={{ ...inputStyles, ...focusStyles, ...style }}
        {...props}
      />
    </div>
  );
}
