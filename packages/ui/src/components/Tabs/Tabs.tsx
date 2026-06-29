import { useRef, type CSSProperties, type KeyboardEvent } from 'react';
import { Icon, type IconName } from '../Icon';

export interface TabItem {
  /** Unique identifier for the tab */
  id: string;
  /** Label displayed for the tab */
  label: string;
  /** Optional icon shown before the label */
  icon?: IconName;
  /** Whether the tab is disabled */
  disabled?: boolean;
}

export interface TabsProps {
  /** Available tabs */
  tabs: TabItem[];
  /** Currently active tab id */
  activeTab: string;
  /** Callback when the active tab changes */
  onChange: (id: string) => void;
  /** Visual variant */
  variant?: 'line' | 'pill';
  /** Size variant */
  size?: 'sm' | 'md' | 'lg';
  /** Stretch tabs to fill the available width */
  fullWidth?: boolean;
}

const sizeConfig = {
  sm: { font: 12, padX: 10, padY: 6, gap: 6, icon: 'sm' as const },
  md: { font: 14, padX: 14, padY: 10, gap: 8, icon: 'sm' as const },
  lg: { font: 16, padX: 18, padY: 12, gap: 10, icon: 'md' as const },
};

const lineListStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'stretch',
  gap: '4px',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
};

const pillListStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'stretch',
  gap: '4px',
  padding: '4px',
  backgroundColor: 'var(--lumik-surface-container, #201f1f)',
  borderRadius: 'var(--lumik-radius-md, 8px)',
};

export function Tabs({
  tabs,
  activeTab,
  onChange,
  variant = 'line',
  size = 'md',
  fullWidth = false,
}: TabsProps) {
  const config = sizeConfig[size];
  const tabRefs = useRef<Array<HTMLButtonElement | null>>([]);

  const focusTab = (index: number) => {
    tabRefs.current[index]?.focus();
  };

  // Find the next enabled tab starting from `start`, moving `step` and wrapping around.
  const nextEnabledIndex = (start: number, step: number): number => {
    const count = tabs.length;
    for (let i = 1; i <= count; i++) {
      const candidate = (start + step * i + count * count) % count;
      if (!tabs[candidate].disabled) return candidate;
    }
    return start;
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLButtonElement>, index: number) => {
    let nextIndex: number;

    switch (e.key) {
      case 'ArrowRight':
        nextIndex = nextEnabledIndex(index, 1);
        break;
      case 'ArrowLeft':
        nextIndex = nextEnabledIndex(index, -1);
        break;
      case 'Home':
        nextIndex = nextEnabledIndex(-1, 1);
        break;
      case 'End':
        nextIndex = nextEnabledIndex(0, -1);
        break;
      default:
        return;
    }

    e.preventDefault();
    if (nextIndex !== index && !tabs[nextIndex].disabled) {
      focusTab(nextIndex);
      onChange(tabs[nextIndex].id);
    }
  };

  const getTabStyles = (isActive: boolean, disabled: boolean): CSSProperties => {
    const base: CSSProperties = {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: config.gap,
      padding: `${config.padY}px ${config.padX}px`,
      fontFamily: 'var(--lumik-font-primary, Inter)',
      fontSize: config.font,
      fontWeight: 500,
      whiteSpace: 'nowrap',
      background: 'transparent',
      border: 'none',
      cursor: disabled ? 'not-allowed' : 'pointer',
      opacity: disabled ? 0.4 : 1,
      transition: 'var(--lumik-transition-fast, 150ms ease)',
      color: isActive
        ? 'var(--lumik-primary, #b0c6ff)'
        : 'var(--lumik-on-surface-variant, #c2c6d7)',
      ...(fullWidth && { flex: 1 }),
    };

    if (variant === 'line') {
      return {
        ...base,
        // Reserve space for the indicator so the row height is stable
        borderBottom: '2px solid',
        borderBottomColor: isActive ? 'var(--lumik-primary, #b0c6ff)' : 'transparent',
        marginBottom: '-1px',
        borderRadius: 0,
      };
    }

    // pill
    return {
      ...base,
      borderRadius: 'var(--lumik-radius-sm, 4px)',
      backgroundColor: isActive
        ? 'var(--lumik-surface-container-high, #2a2a2a)'
        : 'transparent',
    };
  };

  return (
    <div
      role="tablist"
      style={{
        ...(variant === 'line' ? lineListStyles : pillListStyles),
        ...(fullWidth && { width: '100%' }),
      }}
    >
      {tabs.map((tab, index) => {
        const isActive = tab.id === activeTab;
        return (
          <button
            key={tab.id}
            ref={(el) => {
              tabRefs.current[index] = el;
            }}
            type="button"
            role="tab"
            id={`tab-${tab.id}`}
            aria-selected={isActive}
            aria-controls={`tabpanel-${tab.id}`}
            aria-disabled={tab.disabled || undefined}
            tabIndex={isActive ? 0 : -1}
            disabled={tab.disabled}
            style={getTabStyles(isActive, !!tab.disabled)}
            onClick={() => !tab.disabled && onChange(tab.id)}
            onKeyDown={(e) => handleKeyDown(e, index)}
          >
            {tab.icon && <Icon name={tab.icon} size={config.icon} />}
            <span>{tab.label}</span>
          </button>
        );
      })}
    </div>
  );
}
