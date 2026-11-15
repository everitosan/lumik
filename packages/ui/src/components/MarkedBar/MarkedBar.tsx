import type { CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '../Button';
import { Icon, type IconName } from '../Icon';

export interface MarkedBarAction {
  icon: IconName;
  label: string;
  onClick: () => void;
  danger?: boolean;
  disabled?: boolean;
}

export interface MarkedBarProps {
  /** Cuántos hay marcados. */
  count: number;
  /** "N fotos marcadas" o "N elementos marcados". */
  noun: 'photos' | 'items';
  actions: MarkedBarAction[];
  onClear: () => void;
  /** Texto de una acción en curso (p. ej. "Moviendo…"): desactiva los botones. */
  busy?: string | null;
  style?: CSSProperties;
}

const barStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '12px',
  padding: '10px 16px',
  backgroundColor: 'var(--lumik-surface-container-high, #2a2a2a)',
  borderTop: '1px solid var(--lumik-outline-variant, #424654)',
  fontFamily: 'var(--lumik-font-primary, Inter)',
};

const countStyles: CSSProperties = {
  fontSize: '14px',
  fontWeight: 600,
  color: 'var(--lumik-primary, #b0c6ff)',
};

/** Barra de acciones mientras hay elementos marcados (spec photo-marking). */
export function MarkedBar({ count, noun, actions, onClear, busy, style }: MarkedBarProps) {
  const { t } = useTranslation();
  return (
    <div style={{ ...barStyles, ...style }} role="toolbar" aria-label={t(`components.markedBar.${noun}`, { count })}>
      <span style={countStyles}>{t(`components.markedBar.${noun}`, { count })}</span>
      {busy && <span style={{ fontSize: '13px', color: 'var(--lumik-on-surface-variant, #c2c6d7)' }}>{busy}</span>}
      <div style={{ flex: 1 }} />
      {actions.map((a) => (
        <Button
          key={a.label}
          variant={a.danger ? 'danger' : 'secondary'}
          size="sm"
          leftIcon={<Icon name={a.icon} size="sm" />}
          onClick={a.onClick}
          disabled={!!busy || a.disabled}
        >
          {a.label}
        </Button>
      ))}
      <Button variant="ghost" size="sm" onClick={onClear} disabled={!!busy} aria-label={t('components.markedBar.clear')} title={t('components.markedBar.clear')}>
        <Icon name="x" size="sm" />
      </Button>
    </div>
  );
}
