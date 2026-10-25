import { useState, type CSSProperties, type Ref } from 'react';
import { useTranslation } from 'react-i18next';
import { Icon } from '@lumik/ui';
import * as api from '../../lib/api';
import type { SearchQuery } from '../../lib/types';
import { AdvancedSearchPanel } from './AdvancedSearchPanel';

/**
 * ¿El texto parece llevar operadores (`palabra:`, `palabra>=`, `palabra=4`…)? Solo decide si
 * el dashboard deja de filtrar proyectos mientras se escribe; qué operador es
 * válido lo decide el núcleo al buscar.
 */
export function looksLikeOperators(text: string): boolean {
  return /(^|\s)-?[\p{L}]+[:<>=]/u.test(text);
}

const wrapperStyles: CSSProperties = { position: 'relative', width: '100%', maxWidth: '640px' };
const boxStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '6px',
  padding: '2px 4px 2px 14px',
  background: 'var(--lumik-surface-container-high, #2a2a2a)',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: '8px',
};
const inputStyles: CSSProperties = {
  flex: 1,
  minWidth: 0,
  padding: '10px 4px',
  background: 'transparent',
  border: 'none',
  outline: 'none',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
};
const iconButtonStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: '34px',
  height: '34px',
  background: 'transparent',
  border: 'none',
  borderRadius: '6px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  cursor: 'pointer',
};
const hintStyles: CSSProperties = {
  position: 'absolute',
  top: '100%',
  left: '14px',
  marginTop: '4px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '12px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  whiteSpace: 'nowrap',
};

interface Props {
  value: string;
  onChange: (value: string) => void;
  /** Buscar fotos con `query` (Enter, o "Buscar" en el panel). */
  onSearch: (query: string) => void;
  inputRef?: Ref<HTMLInputElement>;
}

/**
 * Buscador del dashboard (spec `photo-search`): al escribir filtra proyectos;
 * Enter, los operadores o el panel avanzado buscan fotos en todos los proyectos.
 */
export function SearchBox({ value, onChange, onSearch, inputRef }: Props) {
  const { t, i18n } = useTranslation();
  const lang: 'es' | 'en' = i18n.language.startsWith('en') ? 'en' : 'es';
  const [panelQuery, setPanelQuery] = useState<SearchQuery | null>(null);

  const togglePanel = async () => {
    if (panelQuery) setPanelQuery(null);
    else setPanelQuery(await api.parseSearchQuery(value));
  };

  const searchFromPanel = async (q: SearchQuery) => {
    const text = await api.formatSearchQuery(q, lang);
    onChange(text);
    setPanelQuery(null);
    if (text) onSearch(text);
  };

  return (
    <div style={wrapperStyles}>
      <form style={boxStyles} onSubmit={(e) => { e.preventDefault(); if (value.trim()) onSearch(value); }}>
        <Icon name="search" size={18} color="var(--lumik-on-surface-variant, #c2c6d7)" />
        <input
          ref={inputRef}
          style={inputStyles}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t('dashboard.searchPlaceholder')}
          aria-label={t('dashboard.searchPlaceholder')}
          spellCheck={false}
        />
        {value && (
          <button type="button" style={iconButtonStyles} aria-label={t('search.clear')} onClick={() => onChange('')}>
            <Icon name="x" size={16} />
          </button>
        )}
        <button
          type="button"
          style={iconButtonStyles}
          aria-label={t('search.advanced')}
          title={t('search.advanced')}
          aria-expanded={panelQuery !== null}
          onClick={() => void togglePanel()}
        >
          <Icon name="chevron-down" size={18} />
        </button>
      </form>
      {value.trim() && !panelQuery && <span style={hintStyles}>{t('search.enterHint')}</span>}
      {panelQuery && (
        <div style={{ position: 'absolute', zIndex: 30, top: 'calc(100% + 8px)', left: '50%', transform: 'translateX(-50%)', width: 'min(880px, 90vw)' }}>
          <AdvancedSearchPanel initial={panelQuery} onSearch={searchFromPanel} onClose={() => setPanelQuery(null)} />
        </div>
      )}
    </div>
  );
}
