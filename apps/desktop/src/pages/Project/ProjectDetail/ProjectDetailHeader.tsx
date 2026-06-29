import type { CSSProperties } from 'react';
import { Icon } from '@lumik/ui';


export interface ProjectDetailHeaderProps {
  projectName: string;
  onBack?: () => void;
  viewMode: 'grid' | 'by-date';
  onViewModeChange: (mode: 'grid' | 'by-date') => void;
  onImport?: () => void;
}

const headerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '16px',
  height: '52px',
  padding: '0 32px',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
  flexShrink: 0,
  boxSizing: 'border-box',
};

const breadcrumbStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  flex: 1,
  minWidth: 0,
};

const breadcrumbLinkStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  background: 'none',
  border: 'none',
  padding: 0,
  cursor: 'pointer',
  flexShrink: 0,
};

const breadcrumbSepStyles: CSSProperties = {
  color: 'var(--lumik-outline, #8c90a0)',
  flexShrink: 0,
};

const breadcrumbCurrentStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};

const controlsStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  flexShrink: 0,
};


export function ProjectDetailHeader({
  projectName,
  onBack,
  viewMode,
  onViewModeChange,
  onImport,
}: ProjectDetailHeaderProps) {

  return (
    <header style={headerStyles}>
      <nav style={breadcrumbStyles} aria-label="breadcrumb">
        <button style={breadcrumbLinkStyles} onClick={onBack}>
          Projects
        </button>
        <span style={breadcrumbSepStyles}>
          <Icon name="chevron-right" size={14} />
        </span>
        <span style={breadcrumbCurrentStyles} title={projectName}>
          {projectName}
        </span>
      </nav>

      <div style={controlsStyles}>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '0',
            border: '1px solid var(--lumik-outline-variant, #424654)',
            borderRadius: 'var(--lumik-radius-sm, 4px)',
            overflow: 'hidden',
          }}
        >
          <button
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '6px 10px',
              fontFamily: 'var(--lumik-font-primary, Inter)',
              fontSize: '13px',
              color: viewMode === 'by-date' ? 'var(--lumik-on-surface, #e5e2e1)' : 'var(--lumik-on-surface-variant, #c2c6d7)',
              background: viewMode === 'by-date' ? 'rgba(255, 255, 255, 0.1)' : 'transparent',
              border: 'none',
              borderRadius: '3px',
              cursor: 'pointer',
              transition: 'all 0.15s',
            }}
            onClick={() => onViewModeChange('by-date')}
            title="Group by date"
          >
            <Icon name="calendar" size={14} />
          </button>

          <button
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              padding: '6px 10px',
              fontFamily: 'var(--lumik-font-primary, Inter)',
              fontSize: '13px',
              color: viewMode === 'grid' ? 'var(--lumik-on-surface, #e5e2e1)' : 'var(--lumik-on-surface-variant, #c2c6d7)',
              background: viewMode === 'grid' ? 'rgba(255, 255, 255, 0.1)' : 'transparent',
              border: 'none',
              borderRadius: '3px',
              cursor: 'pointer',
              transition: 'all 0.15s',
            }}
            onClick={() => onViewModeChange('grid')}
            title="Grid view"
          >
            <Icon name="projects" size={14} />
          </button>
        </div>

        <button
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '6px',
            padding: '6px 12px',
            fontFamily: 'var(--lumik-font-primary, Inter)',
            fontSize: '13px',
            color: 'var(--lumik-on-primary, #fff)',
            background: 'var(--lumik-primary, #1a73e8)',
            border: 'none',
            borderRadius: 'var(--lumik-radius-sm, 4px)',
            cursor: 'pointer',
            whiteSpace: 'nowrap',
          }}
          onClick={onImport}
        >
          <Icon name="import" size={14} />
          Import
        </button>
      </div>
    </header>
  );
}
