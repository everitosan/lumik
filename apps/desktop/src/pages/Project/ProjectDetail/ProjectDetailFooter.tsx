import type { CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { FilterBar, FilterBarDivider, FilterToggle } from '@lumik/ui';
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
    <FilterBar
      start={
        <>
          <div style={statsStyles}>
            <span style={statValueStyles}>{totalPhotos.toLocaleString()}</span>
            <span>{t('projectDetail.photos')}</span>
            <span style={sepStyles}>•</span>
            <span style={culledValueStyles}>{culledCount}</span>
            <span>{t('projectDetail.culled')}</span>
          </div>

          <FilterToggle
            label={t('projectDetail.showCulled')}
            checked={showCulledOnly}
            onChange={onShowCulledOnlyChange}
          />
        </>
      }
      end={
        <>
          <RatingFilter
            value={minStars}
            onChange={onMinStarsChange}
            mode={starsFilterMode}
            onModeChange={onStarsFilterModeChange}
          />

          <FilterBarDivider />

          <ColorsFilter
            selectedColors={selectedColors}
            onChange={onSelectedColorsChange}
          />

          <FilterBarDivider />

          <TagsFilter
            allTags={allTags}
            selectedTags={selectedTags}
            onChange={onSelectedTagsChange}
          />
        </>
      }
    />
  );
}
