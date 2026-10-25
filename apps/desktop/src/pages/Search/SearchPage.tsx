import { useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Button, Icon, Modal, PhotoCard, PHOTO_CARD_HEADER_HEIGHT, PHOTO_CARD_FOOTER_HEIGHT, photoFormat, type ColorLabel } from '@lumik/ui';
import * as api from '../../lib/api';
import type { SearchHit, SearchQuery, SearchResult } from '../../lib/types';
import { AdvancedSearchPanel } from './AdvancedSearchPanel';

const CARD_MIN_WIDTH = 200;
const COL_GAP = 16;
const ROW_GAP = 16;
const CARD_H = PHOTO_CARD_HEADER_HEIGHT + PHOTO_CARD_FOOTER_HEIGHT;
const GROUP_H = 64;

const containerStyles: CSSProperties = { display: 'flex', flexDirection: 'column', height: '100%', overflow: 'hidden' };
const headerStyles: CSSProperties = {
  position: 'relative',
  padding: '24px 32px 16px',
  borderBottom: '1px solid var(--lumik-outline-variant, #424654)',
};
const boxStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '8px',
  maxWidth: '960px',
  padding: '4px 4px 4px 14px',
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
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  fontSize: '14px',
};
const iconButtonStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  width: '36px',
  height: '36px',
  background: 'transparent',
  border: 'none',
  borderRadius: '6px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  cursor: 'pointer',
};
const feedbackStyles: CSSProperties = {
  padding: '48px 32px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  lineHeight: 1.6,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  maxWidth: '640px',
};
const codeStyles: CSSProperties = {
  fontFamily: 'var(--lumik-font-mono, "JetBrains Mono", monospace)',
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

/** Ejemplos del estado vacío, con los operadores del idioma de la app. */
const EXAMPLES_ES = ['tag:calle estrellas>=4 en:2025', 'es:aprobada camara:"X-T5" -tag:boda'];
const EXAMPLES_EN = ['tag:street stars>=4 in:2025', 'is:approved camera:"X-T5" -tag:wedding'];

type Row =
  | { type: 'group'; hit: SearchHit; count: number }
  | { type: 'photos'; hits: SearchHit[] };

/** Agrupa por proyecto, conservando el orden (de la foto más reciente a la más antigua). */
function groupByProject(hits: SearchHit[]): Map<string, SearchHit[]> {
  const groups = new Map<string, SearchHit[]>();
  for (const h of hits) {
    const g = groups.get(h.project_id);
    if (g) g.push(h);
    else groups.set(h.project_id, [h]);
  }
  return groups;
}

function SearchCard({ hit, cache, onOpen }: { hit: SearchHit; cache: Map<string, string>; onOpen: () => void }) {
  const [url, setUrl] = useState<string | null>(cache.get(hit.photo_id) ?? null);
  useEffect(() => {
    if (url) return;
    let active = true;
    api.getSearchThumbnail(hit.photo_id).then((data) => {
      if (active && data) {
        cache.set(hit.photo_id, data);
        setUrl(data);
      }
    });
    return () => { active = false; };
  }, [hit.photo_id, url, cache]);
  return (
    <div style={{ opacity: hit.connected ? 1 : 0.75 }}>
      <PhotoCard
        filename={hit.file_name}
        thumbnailUrl={url ?? undefined}
        stars={hit.stars}
        culled={hit.culled}
        captureDate={hit.capture_date ?? undefined}
        colorLabels={hit.colors.filter((c) => c >= 1 && c <= 5) as ColorLabel[]}
        format={hit.is_pair ? 'RAW+JPG' : photoFormat(hit.file_name)}
        onClick={onOpen}
      />
    </div>
  );
}

interface Props {
  /** Abre una foto de un proyecto conectado en su visor. */
  onOpenPhoto: (projectId: string, photoId: string) => void;
}

/** Búsqueda de fotos en todos los proyectos (spec `photo-search`). */
export function SearchPage({ onOpenPhoto }: Props) {
  const { t, i18n } = useTranslation();
  const lang: 'es' | 'en' = i18n.language.startsWith('en') ? 'en' : 'es';
  const [query, setQuery] = useState('');
  const [result, setResult] = useState<SearchResult | null>(null);
  const [searching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [panelQuery, setPanelQuery] = useState<SearchQuery | null>(null);
  const [indexCount, setIndexCount] = useState<number | null>(null);
  const [offline, setOffline] = useState<SearchHit | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const thumbCache = useRef(new Map<string, string>());

  useEffect(() => {
    inputRef.current?.focus();
    api.getSearchIndexCount().then(setIndexCount).catch(() => setIndexCount(null));
  }, []);

  const run = async (q: string) => {
    if (!q.trim()) {
      setResult(null);
      return;
    }
    setSearching(true);
    setError(null);
    try {
      setResult(await api.searchPhotos(q));
    } catch (err) {
      setError(String(err));
    } finally {
      setSearching(false);
    }
  };

  const openPanel = async () => setPanelQuery(await api.parseSearchQuery(query));

  const searchFromPanel = async (q: SearchQuery) => {
    const text = await api.formatSearchQuery(q, lang);
    setQuery(text);
    setPanelQuery(null);
    await run(text);
  };

  // ── Resultados virtualizados ─────────────────────────────────────────────
  const [scrollEl, setScrollEl] = useState<HTMLDivElement | null>(null);
  const [width, setWidth] = useState(0);
  useEffect(() => {
    if (!scrollEl) return;
    const ro = new ResizeObserver(([entry]) => setWidth(entry.contentRect.width));
    ro.observe(scrollEl);
    return () => ro.disconnect();
  }, [scrollEl]);
  const cols = width > 0 ? Math.max(1, Math.floor((width + COL_GAP) / (CARD_MIN_WIDTH + COL_GAP))) : 4;
  const cardWidth = width > 0 ? (width - (cols - 1) * COL_GAP) / cols : CARD_MIN_WIDTH;
  const rowHeight = cardWidth * 0.75 + CARD_H + ROW_GAP;

  const rows = useMemo((): Row[] => {
    if (!result) return [];
    const out: Row[] = [];
    for (const hits of groupByProject(result.hits).values()) {
      out.push({ type: 'group', hit: hits[0], count: hits.length });
      for (let i = 0; i < hits.length; i += cols) out.push({ type: 'photos', hits: hits.slice(i, i + cols) });
    }
    return out;
  }, [result, cols]);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollEl,
    estimateSize: (i) => (rows[i]?.type === 'group' ? GROUP_H : rowHeight),
    overscan: 3,
  });
  useEffect(() => { virtualizer.measure(); }, [rowHeight, rows.length]); // eslint-disable-line react-hooks/exhaustive-deps

  const openHit = (hit: SearchHit) => (hit.connected ? onOpenPhoto(hit.project_id, hit.photo_id) : setOffline(hit));

  const projectCount = result ? groupByProject(result.hits).size : 0;

  return (
    <div style={containerStyles}>
      <div style={headerStyles}>
        <form style={boxStyles} onSubmit={(e) => { e.preventDefault(); void run(query); }}>
          <Icon name="search" size={18} />
          <input
            ref={inputRef}
            style={inputStyles}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder={t('search.placeholder')}
            aria-label={t('search.title')}
            spellCheck={false}
          />
          {query && (
            <button type="button" style={iconButtonStyles} aria-label={t('search.clear')} onClick={() => { setQuery(''); setResult(null); inputRef.current?.focus(); }}>
              <Icon name="x" size={16} />
            </button>
          )}
          <button type="button" style={iconButtonStyles} aria-label={t('search.advanced')} title={t('search.advanced')} aria-expanded={panelQuery !== null} onClick={() => (panelQuery ? setPanelQuery(null) : void openPanel())}>
            <Icon name="chevron-down" size={18} />
          </button>
          <Button variant="primary" type="submit" disabled={searching}>
            {searching ? t('common.loading') : t('search.submit')}
          </Button>
        </form>
        {panelQuery && (
          <div style={{ position: 'absolute', zIndex: 20, left: '32px', right: '32px', top: '76px', maxWidth: '960px' }}>
            <AdvancedSearchPanel initial={panelQuery} onSearch={searchFromPanel} onClose={() => setPanelQuery(null)} />
          </div>
        )}
        {result && (
          <div style={{ marginTop: '10px', fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '13px', color: 'var(--lumik-on-surface-variant, #c2c6d7)' }}>
            {t('search.summary', { count: result.hits.length })} · {t('search.inProjects', { count: projectCount })}
            {result.truncated && ` · ${t('search.truncated')}`}
          </div>
        )}
      </div>

      <div ref={setScrollEl} style={{ flex: 1, overflowY: 'auto', padding: '8px 32px 24px' }}>
        {error && <div style={feedbackStyles}>{t('search.error', { error })}</div>}

        {!error && !result && (
          <div style={feedbackStyles}>
            {indexCount === 0 ? (
              <p style={{ margin: 0 }}>{t('search.emptyIndex')}</p>
            ) : (
              <>
                <p style={{ marginTop: 0 }}>{t('search.hint', { count: indexCount ?? 0 })}</p>
                <p style={{ margin: 0 }}>
                  {(lang === 'en' ? EXAMPLES_EN : EXAMPLES_ES).map((example) => (
                    <span key={example} style={{ ...codeStyles, display: 'block' }}>{example}</span>
                  ))}
                </p>
              </>
            )}
          </div>
        )}

        {!error && result && result.hits.length === 0 && <div style={feedbackStyles}>{t('search.noResults')}</div>}

        {!error && rows.length > 0 && (
          <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
            {virtualizer.getVirtualItems().map((v) => {
              const row = rows[v.index];
              if (!row) return null;
              const base: CSSProperties = { position: 'absolute', top: 0, left: 0, width: '100%', transform: `translateY(${v.start}px)` };
              if (row.type === 'group') {
                return (
                  <div key={v.key} style={{ ...base, height: GROUP_H, display: 'flex', alignItems: 'flex-end', gap: '12px', paddingBottom: '12px', boxSizing: 'border-box', fontFamily: 'var(--lumik-font-primary, Inter)' }}>
                    <span style={{ fontSize: '17px', fontWeight: 600, color: 'var(--lumik-on-surface, #e5e2e1)' }}>{row.hit.project_name}</span>
                    {row.hit.project_date && <span style={{ fontSize: '13px', color: 'var(--lumik-on-surface-variant, #c2c6d7)' }}>{row.hit.project_date}</span>}
                    <span style={{ fontSize: '13px', color: 'var(--lumik-on-surface-variant, #c2c6d7)' }}>{t('search.photos', { count: row.count })}</span>
                    {!row.hit.connected && (
                      <span style={{ display: 'inline-flex', alignItems: 'center', gap: '4px', fontSize: '12px', color: 'var(--lumik-secondary, #e9c349)' }}>
                        <Icon name="drive" size="sm" aria-hidden="true" />
                        {row.hit.device_name ? t('search.notConnected', { device: row.hit.device_name }) : t('search.notConnectedUnknown')}
                      </span>
                    )}
                  </div>
                );
              }
              return (
                <div key={v.key} style={{ ...base, display: 'grid', gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))`, gap: `${COL_GAP}px`, paddingBottom: `${ROW_GAP}px` }}>
                  {row.hits.map((hit) => (
                    <SearchCard key={hit.photo_id} hit={hit} cache={thumbCache.current} onOpen={() => openHit(hit)} />
                  ))}
                </div>
              );
            })}
          </div>
        )}
      </div>

      <Modal title={t('search.offlineTitle')} open={offline !== null} onClose={() => setOffline(null)} closable style={{ width: '440px' }}>
        {offline && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '14px', lineHeight: 1.5 }}>
            {thumbCache.current.get(offline.photo_id) && (
              <img src={thumbCache.current.get(offline.photo_id)} alt={offline.file_name} style={{ alignSelf: 'center', maxWidth: '100%', borderRadius: '6px' }} />
            )}
            <p style={{ margin: 0 }}>
              {offline.device_name
                ? t('search.offlineMessage', { project: offline.project_name, device: offline.device_name })
                : t('search.offlineMessageUnknown', { project: offline.project_name })}
            </p>
            <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
              <Button variant="primary" onClick={() => setOffline(null)}>{t('search.offlineOk')}</Button>
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
}
