import type { CSSProperties, ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '../Icon';

export interface CollapsibleSectionProps {
  title: string;
  /** Shown at the right of the header, mainly so a collapsed section still
   *  tells what is going on inside (e.g. "2 · 4/43"). */
  summary?: ReactNode;
  /** Controlled: the parent owns (and persists) the collapsed state. */
  collapsed: boolean;
  onToggle: () => void;
  children?: ReactNode;
  /** Applied to the content wrapper, e.g. to cap its height and scroll. */
  contentStyle?: CSSProperties;
  className?: string;
  style?: CSSProperties;
}

const containerStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  minHeight: 0,
};

const headerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  width: '100%',
  padding: '8px 8px 4px',
  border: 'none',
  background: 'transparent',
  cursor: 'pointer',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  textAlign: 'left',
};

const titleStyles: CSSProperties = {
  flex: 1,
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '11px',
  fontWeight: 600,
  textTransform: 'uppercase',
  letterSpacing: '0.05em',
};

const summaryStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '11px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  whiteSpace: 'nowrap',
};

const contentStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
};

export function CollapsibleSection({
  title,
  summary,
  collapsed,
  onToggle,
  children,
  contentStyle,
  className,
  style,
}: CollapsibleSectionProps) {
  const { t } = useTranslation();
  return (
    <section style={{ ...containerStyles, ...style }} className={className}>
      <button
        type="button"
        style={headerStyles}
        onClick={onToggle}
        aria-expanded={!collapsed}
        aria-label={t(collapsed ? 'components.collapsibleSection.expand' : 'components.collapsibleSection.collapse', { title })}
      >
        <Icon name={collapsed ? 'chevron-right' : 'chevron-down'} size={14} />
        <span style={titleStyles}>{title}</span>
        {summary != null && summary !== '' && <span style={summaryStyles}>{summary}</span>}
      </button>
      {!collapsed && <div style={{ ...contentStyles, ...contentStyle }}>{children}</div>}
    </section>
  );
}
