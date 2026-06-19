import type { CSSProperties } from 'react';
import { Icon } from '../Icon';

export type WorkflowStatus = 'imported' | 'edited' | 'delivered';

export interface ProjectCardProps {
  name: string;
  photoCount: number;
  date: string;
  driveName: string;
  status: WorkflowStatus;
  thumbnailUrl?: string;
  onClick?: () => void;
  onMenuClick?: () => void;
  className?: string;
  style?: CSSProperties;
}

const cardStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  backgroundColor: 'var(--lumik-glass-bg, rgba(32, 31, 31, 0.6))',
  backdropFilter: 'blur(var(--lumik-glass-blur, 12px))',
  border: '1px solid var(--lumik-glass-border, rgba(66, 70, 84, 0.3))',
  borderRadius: 'var(--lumik-radius-md, 8px)',
  overflow: 'hidden',
  cursor: 'pointer',
  transition: 'transform var(--lumik-transition-normal, 250ms ease), box-shadow var(--lumik-transition-normal, 250ms ease)',
};

const thumbnailContainerStyles: CSSProperties = {
  position: 'relative',
  aspectRatio: '4/3',
  backgroundColor: 'var(--lumik-surface-container-lowest, #0e0e0e)',
  overflow: 'hidden',
};

const thumbnailStyles: CSSProperties = {
  width: '100%',
  height: '100%',
  objectFit: 'cover',
};

const placeholderStyles: CSSProperties = {
  width: '100%',
  height: '100%',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  color: 'var(--lumik-outline, #8c90a0)',
};

const contentStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
  padding: '12px',
};

const headerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'flex-start',
  justifyContent: 'space-between',
  gap: '8px',
};

const titleStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '16px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  margin: 0,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};

const menuButtonStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: '24px',
  height: '24px',
  padding: 0,
  backgroundColor: 'transparent',
  border: 'none',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  cursor: 'pointer',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  flexShrink: 0,
};

const metaContainerStyles: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: '1fr 1fr',
  gap: '4px 8px',
  fontFamily: 'var(--lumik-font-mono, JetBrains Mono)',
  fontSize: '12px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const metaItemStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '4px',
  overflow: 'hidden',
};

const metaItemFullStyles: CSSProperties = {
  ...metaItemStyles,
  gridColumn: '1 / -1',
};

const statusBadgeStyles: Record<WorkflowStatus, CSSProperties> = {
  imported: {
    padding: '2px 8px',
    fontSize: '11px',
    fontWeight: 500,
    borderRadius: 'var(--lumik-radius-full, 9999px)',
    backgroundColor: 'rgba(176, 198, 255, 0.15)',
    color: 'var(--lumik-primary, #b0c6ff)',
  },
  edited: {
    padding: '2px 8px',
    fontSize: '11px',
    fontWeight: 500,
    borderRadius: 'var(--lumik-radius-full, 9999px)',
    backgroundColor: 'rgba(233, 195, 73, 0.15)',
    color: 'var(--lumik-secondary, #e9c349)',
  },
  delivered: {
    padding: '2px 8px',
    fontSize: '11px',
    fontWeight: 500,
    borderRadius: 'var(--lumik-radius-full, 9999px)',
    backgroundColor: 'rgba(255, 182, 144, 0.15)',
    color: 'var(--lumik-tertiary, #ffb690)',
  },
};

const statusLabels: Record<WorkflowStatus, string> = {
  imported: 'Imported',
  edited: 'Editing',
  delivered: 'Delivered',
};

function formatDate(dateString: string): string {
  // Date-only strings (YYYY-MM-DD) are parsed as UTC midnight by spec, which
  // shifts the date back one day in negative-offset timezones. Appending
  // T00:00:00 forces local-time interpretation.
  const normalized = /^\d{4}-\d{2}-\d{2}$/.test(dateString)
    ? dateString + 'T00:00:00'
    : dateString;
  const date = new Date(normalized);
  if (isNaN(date.getTime())) return dateString;
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  return `${year}/${month}/${day}`;
}

export function ProjectCard({
  name,
  photoCount,
  date,
  driveName,
  status,
  thumbnailUrl,
  onClick,
  onMenuClick,
  className,
  style,
}: ProjectCardProps) {
  const handleMenuClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    onMenuClick?.();
  };

  return (
    <div
      style={{ ...cardStyles, ...style }}
      className={className}
      onClick={onClick}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClick?.(); } }}
      role="button"
      tabIndex={0}
    >
      <div style={thumbnailContainerStyles}>
        {thumbnailUrl ? (
          <img src={thumbnailUrl} alt={name} style={thumbnailStyles} />
        ) : (
          <div style={placeholderStyles}>
            <Icon name="image" size={48} />
          </div>
        )}
      </div>

      <div style={contentStyles}>
        <div style={headerStyles}>
          <h3 style={titleStyles} title={name}>
            {name}
          </h3>
          <button
            style={menuButtonStyles}
            onClick={handleMenuClick}
            aria-label="Project options"
          >
            <Icon name="menu" size="md" />
          </button>
        </div>

        <div style={metaContainerStyles}>
          <span style={{ ...metaItemFullStyles, overflow: 'hidden' }}>
            <Icon name="drive" size="sm" />
            <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
              {driveName}
            </span>
          </span>
          <span style={metaItemStyles}>
            <Icon name="calendar" size="sm" />
            {formatDate(date)}
          </span>
          <span style={{ ...metaItemStyles, justifyContent: 'flex-end' }}>
            <Icon name="photo" size="sm" />
            {photoCount} photos
          </span>
        </div>

        <div>
          <span style={statusBadgeStyles[status]}>{statusLabels[status]}</span>
        </div>
      </div>
    </div>
  );
}
