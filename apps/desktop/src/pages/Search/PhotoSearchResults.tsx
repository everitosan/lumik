import { useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Button, Icon, Modal, PhotoCard, PHOTO_CARD_HEADER_HEIGHT, PHOTO_CARD_FOOTER_HEIGHT, photoFormat, type ColorLabel } from '@lumik/ui';
import * as api from '../../lib/api';
import type { SearchHit, SearchResult } from '../../lib/types';

const CARD_MIN_WIDTH = 200;
const COL_GAP = 16;
const ROW_GAP = 16;
const CARD_H = PHOTO_CARD_HEADER_HEIGHT + PHOTO_CARD_FOOTER_HEIGHT;
const GROUP_H = 64;

const feedbackStyles: CSSProperties = {
  padding: '48px 0',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  lineHeight: 1.6,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};
const summaryStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  flexWrap: 'wrap',
  gap: '8px 16px',
  paddingBottom: '8px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

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
  result: SearchResult | null;
  searching: boolean;
  error: string | null;
  /** Posición a la que volver (al regresar del visor). */
  initialScrollTop: number;
  /** Se llama justo antes de abrir una foto, con la posición actual. */
  onOpenPhoto: (projectId: string, photoId: string, scrollTop: number) => void;
  /** Vuelve al grid de proyectos. */
  onClose: () => void;
}

/**
 * Resultados de la búsqueda de fotos (spec `photo-search`): agrupados por
 * proyecto y virtualizados. Una foto cuyo disco no está conectado muestra un
 * aviso con su miniatura y el disco que hay que conectar.
 */
export function PhotoSearchResults({ result, searching, error, initialScrollTop, onOpenPhoto, onClose }: Props) {
  const { t } = useTranslation();
  const [offline, setOffline] = useState<SearchHit | null>(null);
  const thumbCache = useRef(new Map<string, string>());

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

  const groups = useMemo(() => groupByProject(result?.hits ?? []), [result]);
  const rows = useMemo((): Row[] => {
    const out: Row[] = [];
    for (const hits of groups.values()) {
      out.push({ type: 'group', hit: hits[0], count: hits.length });
      for (let i = 0; i < hits.length; i += cols) out.push({ type: 'photos', hits: hits.slice(i, i + cols) });
    }
    return out;
  }, [groups, cols]);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollEl,
    estimateSize: (i) => (rows[i]?.type === 'group' ? GROUP_H : rowHeight),
    overscan: 3,
  });
  useEffect(() => { virtualizer.measure(); }, [rowHeight, rows.length]); // eslint-disable-line react-hooks/exhaustive-deps

  // Al volver del visor, la misma posición (una sola vez).
  const restoredScroll = useRef(false);
  useEffect(() => {
    if (restoredScroll.current || !scrollEl || rows.length === 0 || !initialScrollTop) return;
    restoredScroll.current = true;
    scrollEl.scrollTop = initialScrollTop;
  }, [scrollEl, rows.length, initialScrollTop]);

  const openHit = (hit: SearchHit) =>
    hit.connected ? onOpenPhoto(hit.project_id, hit.photo_id, scrollEl?.scrollTop ?? 0) : setOffline(hit);

  return (
    <div style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column' }}>
      <div style={summaryStyles}>
        {result && (
          <span>
            {t('search.summary', { count: result.hits.length })} · {t('search.inProjects', { count: groups.size })}
            {result.truncated && ` · ${t('search.truncated')}`}
          </span>
        )}
        {searching && <span>{t('common.loading')}</span>}
        <Button variant="ghost" size="sm" leftIcon={<Icon name="x" size="sm" />} onClick={onClose}>
          {t('search.backToProjects')}
        </Button>
      </div>

      <div ref={setScrollEl} style={{ flex: 1, overflowY: 'auto', paddingRight: '8px' }}>
        {error && <div style={feedbackStyles}>{t('search.error', { error })}</div>}
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
