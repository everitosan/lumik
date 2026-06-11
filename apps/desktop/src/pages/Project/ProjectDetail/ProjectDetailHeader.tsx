import type { CSSProperties } from 'react';
import { Icon } from '@lumik/ui';

export type SortOption = 'date' | 'name' | 'stars';

const SORT_CYCLE: SortOption[] = ['date', 'name', 'stars'];
const SORT_LABELS: Record<SortOption, string> = {
  date: 'Date',
  name: 'Name',
  stars: 'Stars',
};

export interface ProjectDetailHeaderProps {
  projectName: string;
  onBack?: () => void;
  sortBy: SortOption;
  onSortChange: (sort: SortOption) => void;
  showCulledOnly: boolean;
  onShowCulledOnlyChange: (value: boolean) => void;
}

const headerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '16px',
  padding: '16px 32px',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
  flexShrink: 0,
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

const sortButtonStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  padding: '6px 10px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  background: 'transparent',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  cursor: 'pointer',
  whiteSpace: 'nowrap',
};

export function ProjectDetailHeader({
  projectName,
  onBack,
  sortBy,
  onSortChange,
  showCulledOnly,
  onShowCulledOnlyChange,
}: ProjectDetailHeaderProps) {
  const cycleSort = () => {
    const idx = SORT_CYCLE.indexOf(sortBy);
    onSortChange(SORT_CYCLE[(idx + 1) % SORT_CYCLE.length]);
  };

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
        <label
          onClick={() => onShowCulledOnlyChange(!showCulledOnly)}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: '7px',
            cursor: 'pointer',
            fontFamily: 'var(--lumik-font-primary, Inter)',
            fontSize: '13px',
            color: showCulledOnly ? 'var(--lumik-tertiary, #ffb690)' : 'var(--lumik-on-surface-variant, #c2c6d7)',
            whiteSpace: 'nowrap',
            userSelect: 'none',
          }}
        >
          <span
            role="checkbox"
            aria-checked={showCulledOnly}
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              width: '16px',
              height: '16px',
              borderRadius: '50%',
              border: `2px solid ${showCulledOnly ? 'var(--lumik-tertiary, #ffb690)' : 'var(--lumik-outline-variant, #424654)'}`,
              backgroundColor: showCulledOnly ? 'var(--lumik-tertiary, #ffb690)' : 'transparent',
              flexShrink: 0,
              transition: 'background-color 0.15s, border-color 0.15s',
              cursor: 'pointer',
            }}
          >
            {showCulledOnly && (
              <svg width="8" height="8" viewBox="0 0 8 8" fill="none">
                <path d="M1 4l2 2 4-4" stroke="var(--lumik-on-tertiary, #2d1600)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            )}
          </span>
          Show culled
        </label>

        <button style={sortButtonStyles} onClick={cycleSort}>
          Sort: {SORT_LABELS[sortBy]}
          <Icon name="chevron-down" size={12} />
        </button>

      </div>
    </header>
  );
}
