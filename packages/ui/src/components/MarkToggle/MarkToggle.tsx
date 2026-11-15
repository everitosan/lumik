import type { CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '../Icon';

export interface MarkToggleProps {
  marked: boolean;
  onToggle: () => void;
  style?: CSSProperties;
}

const baseStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: '22px',
  height: '22px',
  padding: 0,
  borderRadius: '6px',
  border: '2px solid rgba(255, 255, 255, 0.85)',
  backgroundColor: 'rgba(0, 0, 0, 0.45)',
  color: 'var(--lumik-on-primary, #002d6f)',
  cursor: 'pointer',
  boxShadow: '0 1px 3px rgba(0, 0, 0, 0.5)',
};

const markedStyles: CSSProperties = {
  backgroundColor: 'var(--lumik-primary, #b0c6ff)',
  borderColor: 'var(--lumik-primary, #b0c6ff)',
};

/** Casilla para marcar una tarjeta (spec photo-marking). No dispara el clic de la tarjeta. */
export function MarkToggle({ marked, onToggle, style }: MarkToggleProps) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={marked}
      aria-label={marked ? t('components.mark.unmark') : t('components.mark.mark')}
      title={marked ? t('components.mark.unmark') : t('components.mark.mark')}
      style={{ ...baseStyles, ...(marked ? markedStyles : null), ...style }}
      onClick={(e) => {
        e.stopPropagation();
        onToggle();
      }}
      onKeyDown={(e) => e.stopPropagation()}
    >
      {marked && <Icon name="check" size={14} />}
    </button>
  );
}
