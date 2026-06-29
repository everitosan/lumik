import { useState, useRef, useEffect, type CSSProperties, type ReactNode } from 'react';
import { Icon } from '../Icon';

export interface SelectOption {
  /** Unique value for the option */
  value: string;
  /** Display label */
  label: string;
  /** Optional icon to show before label */
  icon?: ReactNode;
  /** Whether this is a special action option (styled differently) */
  isAction?: boolean;
}

export interface SelectProps {
  /** Available options */
  options: SelectOption[];
  /** Currently selected value */
  value?: string;
  /** Callback when selection changes */
  onChange?: (value: string, option: SelectOption) => void;
  /** Label displayed above the select */
  label?: string;
  /** Placeholder when no value selected */
  placeholder?: string;
  /** Whether the select is disabled */
  disabled?: boolean;
  /** Full width mode */
  fullWidth?: boolean;
}

const containerStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  position: 'relative',
};

const labelStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  fontWeight: 500,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
};

const getTriggerStyles = (isOpen: boolean, disabled: boolean): CSSProperties => ({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '12px',
  padding: '10px 12px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  border: `1px solid ${
    isOpen
      ? 'var(--lumik-primary, #b0c6ff)'
      : 'var(--lumik-outline-variant, #424654)'
  }`,
  cursor: disabled ? 'not-allowed' : 'pointer',
  opacity: disabled ? 0.5 : 1,
  transition: 'var(--lumik-transition-fast, 150ms ease)',
  outline: 'none',
  width: '100%',
  minWidth: '200px',
});

const triggerTextStyles = (hasValue: boolean): CSSProperties => ({
  flex: 1,
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: hasValue
    ? 'var(--lumik-on-surface, #e5e2e1)'
    : 'var(--lumik-outline, #8c90a0)',
  textAlign: 'left',
  whiteSpace: 'nowrap',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
});

const triggerIconContainerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  flexShrink: 0,
};

const chevronStyles = (isOpen: boolean): CSSProperties => ({
  transition: 'transform 150ms ease',
  transform: isOpen ? 'rotate(180deg)' : 'rotate(0deg)',
});

const dropdownStyles: CSSProperties = {
  position: 'absolute',
  top: '100%',
  left: 0,
  right: 0,
  marginTop: '4px',
  backgroundColor: 'var(--lumik-surface-container-low, #1c1b1b)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  boxShadow: 'var(--lumik-shadow-md, 0 4px 12px rgba(0, 0, 0, 0.4))',
  zIndex: 1000,
  overflow: 'hidden',
  maxHeight: '240px',
  overflowY: 'auto',
};

const getOptionStyles = (isSelected: boolean, isAction: boolean): CSSProperties => ({
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '10px 12px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: isSelected
    ? 'var(--lumik-primary, #b0c6ff)'
    : isAction
      ? 'var(--lumik-primary, #b0c6ff)'
      : 'var(--lumik-on-surface, #e5e2e1)',
  backgroundColor: isSelected
    ? 'rgba(176, 198, 255, 0.1)'
    : 'transparent',
  cursor: 'pointer',
  transition: 'background-color 100ms ease',
});

const optionHoverBg = 'rgba(255, 255, 255, 0.05)';

export function Select({
  options,
  value,
  onChange,
  label,
  placeholder = 'Select...',
  disabled = false,
  fullWidth = false,
}: SelectProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const selectedOption = options.find((opt) => opt.value === value);

  // Close on outside click
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isOpen]);

  // Keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (disabled) return;

    switch (e.key) {
      case 'Enter':
      case ' ':
        e.preventDefault();
        if (isOpen && hoveredIndex !== null) {
          const option = options[hoveredIndex];
          onChange?.(option.value, option);
          setIsOpen(false);
        } else {
          setIsOpen(!isOpen);
        }
        break;
      case 'Escape':
        setIsOpen(false);
        break;
      case 'ArrowDown':
        e.preventDefault();
        if (!isOpen) {
          setIsOpen(true);
        } else {
          setHoveredIndex((prev) =>
            prev === null ? 0 : Math.min(prev + 1, options.length - 1)
          );
        }
        break;
      case 'ArrowUp':
        e.preventDefault();
        if (isOpen) {
          setHoveredIndex((prev) =>
            prev === null ? options.length - 1 : Math.max(prev - 1, 0)
          );
        }
        break;
    }
  };

  const handleSelect = (option: SelectOption) => {
    onChange?.(option.value, option);
    setIsOpen(false);
  };

  return (
    <div
      ref={containerRef}
      style={{
        ...containerStyles,
        ...(fullWidth && { width: '100%' }),
      }}
    >
      {label && <span style={labelStyles}>{label}</span>}

      <div
        role="combobox"
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        tabIndex={disabled ? -1 : 0}
        style={getTriggerStyles(isOpen, disabled)}
        onClick={() => !disabled && setIsOpen(!isOpen)}
        onKeyDown={handleKeyDown}
      >
        <span style={triggerTextStyles(!!selectedOption)}>
          {selectedOption ? (
            <span style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
              {selectedOption.icon}
              {selectedOption.label}
            </span>
          ) : (
            placeholder
          )}
        </span>

        <div style={triggerIconContainerStyles}>
          {selectedOption && (
            <Icon
              name="check"
              size="sm"
              color="var(--lumik-primary, #b0c6ff)"
            />
          )}
          <span style={chevronStyles(isOpen)}>
            <Icon
              name="chevron-down"
              size="sm"
              color="var(--lumik-on-surface-variant, #c2c6d7)"
            />
          </span>
        </div>
      </div>

      {isOpen && (
        <div style={dropdownStyles} role="listbox">
          {options.map((option, index) => (
            <div
              key={option.value}
              role="option"
              aria-selected={option.value === value}
              style={{
                ...getOptionStyles(option.value === value, !!option.isAction),
                ...(hoveredIndex === index && { backgroundColor: optionHoverBg }),
              }}
              onClick={() => handleSelect(option)}
              onMouseEnter={() => setHoveredIndex(index)}
              onMouseLeave={() => setHoveredIndex(null)}
            >
              {option.icon}
              {option.label}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
