import type { CSSProperties } from 'react';
import { useState, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '@lumik/ui';

const popoverTriggerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '4px',
  padding: '6px 10px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  background: 'transparent',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  cursor: 'pointer',
  transition: 'all 0.15s',
  whiteSpace: 'nowrap',
};

const popoverTriggerActiveStyles: CSSProperties = {
  ...popoverTriggerStyles,
  color: 'var(--lumik-tertiary, #ffb690)',
  borderColor: 'var(--lumik-tertiary, #ffb690)',
};

const popoverPanelStyles: CSSProperties = {
  position: 'absolute',
  top: '100%',
  right: 0,
  marginTop: '4px',
  backgroundColor: 'var(--lumik-surface-container, #2d2d2d)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  boxShadow: '0 4px 12px rgba(0, 0, 0, 0.4)',
  zIndex: 1000,
  minWidth: '200px',
};

const popoverContentStyles: CSSProperties = {
  padding: '8px 12px',
  display: 'flex',
  flexDirection: 'column',
  gap: '8px',
};

const optionStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  padding: '6px 8px',
  cursor: 'pointer',
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  borderRadius: '4px',
  transition: 'background-color 0.15s',
};

const optionHoverStyles: CSSProperties = {
  backgroundColor: 'var(--lumik-surface-bright, #3a3a3a)',
};

const optionSelectedStyles: CSSProperties = {
  backgroundColor: 'var(--lumik-surface-bright, #3a3a3a)',
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const checkboxStyles: CSSProperties = {
  width: '16px',
  height: '16px',
  borderRadius: '3px',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  flexShrink: 0,
  transition: 'all 0.15s',
};

const checkboxCheckedStyles: CSSProperties = {
  ...checkboxStyles,
  backgroundColor: 'var(--lumik-tertiary, #ffb690)',
  borderColor: 'var(--lumik-tertiary, #ffb690)',
};

interface RatingFilterProps {
  value: number | null;
  onChange: (stars: number | null) => void;
  mode: 'exact' | 'inclusive';
  onModeChange: (mode: 'exact' | 'inclusive') => void;
}

export function RatingFilter({ value, onChange, mode, onModeChange }: RatingFilterProps) {
  const { t } = useTranslation();
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '2px',
        padding: '6px 4px',
      }}
    >
      <button
        onClick={() => {
          onModeChange(mode === 'exact' ? 'inclusive' : 'exact');
        }}
        style={{
          background: 'none',
          border: 'none',
          cursor: 'pointer',
          padding: '0 6px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          fontSize: '14px',
          fontWeight: 700,
          fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
          color: value !== null ? 'var(--lumik-tertiary, #ffb690)' : 'var(--lumik-on-surface-variant, #c2c6d7)',
          transition: 'all 0.15s',
          lineHeight: '1',
        }}
        title={t('projectDetail.filterToggle')}
      >
        {mode === 'exact' ? '=' : '≥'}
      </button>

      {[1, 2, 3, 4, 5].map((star) => (
        <button
          key={star}
          onClick={() => {
            onChange(value === star ? null : star);
          }}
          style={{
            background: 'none',
            border: 'none',
            cursor: 'pointer',
            padding: '0',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'all 0.15s',
          }}
          title={mode === 'inclusive' ? t('projectDetail.ratingFilter.upTo', { star }) : t('projectDetail.ratingFilter.exactly', { star })}
        >
          <Icon
            name={value !== null && star <= value ? 'star-filled' : 'star'}
            size={16}
            color={value !== null && star <= value ? 'var(--lumik-tertiary, #ffb690)' : 'var(--lumik-on-surface-variant, #c2c6d7)'}
          />
        </button>
      ))}
    </div>
  );
}

interface TagsFilterProps {
  allTags: Set<string>;
  selectedTags: Set<string>;
  onChange: (tags: Set<string>) => void;
}

const popoverPanelTopStyles: CSSProperties = {
  position: 'absolute',
  bottom: '100%',
  right: 0,
  marginBottom: '4px',
  backgroundColor: 'var(--lumik-surface-container, #2d2d2d)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  boxShadow: '0 4px 12px rgba(0, 0, 0, 0.4)',
  zIndex: 1000,
  minWidth: '200px',
};

export function TagsFilter({ allTags, selectedTags, onChange }: TagsFilterProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    if (open) document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [open]);

  const handleToggleTag = (tag: string) => {
    const newTags = new Set(selectedTags);
    if (newTags.has(tag)) {
      newTags.delete(tag);
    } else {
      newTags.add(tag);
    }
    onChange(newTags);
  };

  const sortedTags = Array.from(allTags).sort();

  return (
    <div style={{ position: 'relative' }} ref={ref}>
      <button
        style={selectedTags.size > 0 ? popoverTriggerActiveStyles : popoverTriggerStyles}
        onClick={() => setOpen(!open)}
      >
        <Icon name="tags" size={14} />
        {selectedTags.size > 0 && `(${selectedTags.size})`}
      </button>
      {open && (
        <div style={popoverPanelTopStyles}>
          <div style={popoverContentStyles}>
            {sortedTags.length === 0 ? (
              <div style={{ padding: '8px', color: 'var(--lumik-on-surface-variant, #c2c6d7)', fontSize: '12px' }}>
                {t('projectDetail.noTagsAvailable')}
              </div>
            ) : (
              sortedTags.map((tag) => (
                <div
                  key={tag}
                  style={selectedTags.has(tag) ? { ...optionStyles, ...optionSelectedStyles } : optionStyles}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.backgroundColor = 'var(--lumik-surface-bright, #3a3a3a)';
                  }}
                  onMouseLeave={(e) => {
                    if (!selectedTags.has(tag)) {
                      e.currentTarget.style.backgroundColor = 'transparent';
                    }
                  }}
                  onClick={() => handleToggleTag(tag)}
                >
                  <div
                    style={selectedTags.has(tag) ? checkboxCheckedStyles : checkboxStyles}
                  >
                    {selectedTags.has(tag) && (
                      <Icon name="check" size={10} color="var(--lumik-on-tertiary, #2d1600)" />
                    )}
                  </div>
                  {tag}
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}

interface ColorsFilterProps {
  selectedColors: Set<number>;
  onChange: (colors: Set<number>) => void;
}

const colorMap: Record<number, { name: string; hex: string }> = {
  1: { name: 'Red', hex: '#ff6b6b' },
  2: { name: 'Yellow', hex: '#ffd93d' },
  3: { name: 'Green', hex: '#6bcf7f' },
  4: { name: 'Blue', hex: '#4d96ff' },
  5: { name: 'Purple', hex: '#d946ef' },
};

export function ColorsFilter({ selectedColors, onChange }: ColorsFilterProps) {
  const handleToggleColor = (colorId: number) => {
    const newColors = new Set(selectedColors);
    if (newColors.has(colorId)) {
      newColors.delete(colorId);
    } else {
      newColors.add(colorId);
    }
    onChange(newColors);
  };

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        padding: '6px 4px',
      }}
    >
      {Object.entries(colorMap).map(([id, color]) => {
        const colorId = parseInt(id, 10);
        return (
          <button
            key={colorId}
            onClick={() => handleToggleColor(colorId)}
            style={{
              width: '18px',
              height: '18px',
              borderRadius: '50%',
              backgroundColor: color.hex,
              border: selectedColors.has(colorId) ? '2px solid var(--lumik-on-surface, #e5e2e1)' : '2px solid transparent',
              cursor: 'pointer',
              padding: '0',
              transition: 'all 0.15s',
              flexShrink: 0,
            }}
            title={color.name}
          />
        );
      })}
    </div>
  );
}
