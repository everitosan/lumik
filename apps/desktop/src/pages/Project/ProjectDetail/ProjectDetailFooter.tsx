import type { CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '@lumik/ui';
import { RatingFilter, TagsFilter, ColorsFilter } from './FilterPopovers';

export interface ProjectDetailFooterProps {
  totalPhotos: number;
  culledCount: number;
  showCulledOnly: boolean;
  onShowCulledOnlyChange: (value: boolean) => void;
  minStars: number | null;
  onMinStarsChange: (stars: number | null) => void;
  starsFilterMode: 'exact' | 'inclusive';
  onStarsFilterModeChange: (mode: 'exact' | 'inclusive') => void;
  allTags: Set<string>;
  selectedTags: Set<string>;
  onSelectedTagsChange: (tags: Set<string>) => void;
  selectedColors: Set<number>;
  onSelectedColorsChange: (colors: Set<number>) => void;
}

const footerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '10px 32px',
  borderTop: '1px solid var(--lumik-outline-variant, #424654)',
  flexShrink: 0,
};

const statsStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '12px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const statValueStyles: CSSProperties = {
  color: 'var(--lumik-on-surface, #e5e2e1)',
  fontWeight: 500,
};

const sepStyles: CSSProperties = {
  color: 'var(--lumik-outline-variant, #424654)',
};

const culledValueStyles: CSSProperties = {
  color: 'var(--lumik-tertiary, #ffb690)',
  fontWeight: 500,
};

export function ProjectDetailFooter({
  totalPhotos,
  culledCount,
  showCulledOnly,
  onShowCulledOnlyChange,
  minStars,
  onMinStarsChange,
  starsFilterMode,
  onStarsFilterModeChange,
  allTags,
  selectedTags,
  onSelectedTagsChange,
  selectedColors,
  onSelectedColorsChange,
}: ProjectDetailFooterProps) {
  const { t } = useTranslation();
  return (
    <footer style={footerStyles}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '16px' }}>
        <div style={statsStyles}>
          <span style={statValueStyles}>{totalPhotos.toLocaleString()}</span>
          <span>{t('projectDetail.photos')}</span>
          <span style={sepStyles}>•</span>
          <span style={culledValueStyles}>{culledCount}</span>
          <span>{t('projectDetail.culled')}</span>
        </div>

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
          {t('projectDetail.showCulled')}
        </label>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
        <RatingFilter
          value={minStars}
          onChange={onMinStarsChange}
          mode={starsFilterMode}
          onModeChange={onStarsFilterModeChange}
        />

        <div
          style={{
            width: '1px',
            height: '20px',
            backgroundColor: 'var(--lumik-outline-variant, #424654)',
            opacity: 0.5,
          }}
        />

        <ColorsFilter
          selectedColors={selectedColors}
          onChange={onSelectedColorsChange}
        />

        <div
          style={{
            width: '1px',
            height: '20px',
            backgroundColor: 'var(--lumik-outline-variant, #424654)',
            opacity: 0.5,
          }}
        />

        <TagsFilter
          allTags={allTags}
          selectedTags={selectedTags}
          onChange={onSelectedTagsChange}
        />
      </div>
    </footer>
  );
}
