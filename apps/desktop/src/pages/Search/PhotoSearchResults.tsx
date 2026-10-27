import { useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Button, Icon, Modal, PhotoCard, PHOTO_CARD_HEADER_HEIGHT, PHOTO_CARD_FOOTER_HEIGHT, photoFormat, type ColorLabel } from '@lumik/ui';
import * as api from '../../lib/api';
import type { SearchHit, SearchResult } from '../../lib/types';
import { AddToAlbumDialog, type AlbumTarget } from '../Albums/AddToAlbumDialog';
import { CollectionViewer, type PhotoEdits } from '../Collection/CollectionViewer';

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

/**
 * Agrupa por proyecto, conservando el orden del núcleo: de la foto más reciente
 * a la más antigua y, al buscar por contenido, después las que se parecen, de
 * más a menos. Cada grupo queda donde salió su primera foto.
 */
function groupByProject(hits: SearchHit[]): Map<string, SearchHit[]> {
  const groups = new Map<string, SearchHit[]>();
  for (const h of hits) {
    const g = groups.get(h.project_id);
    if (g) g.push(h);
    else groups.set(h.project_id, [h]);
  }
  return groups;
}

function SearchCard({ hit, cache, onOpen, onAddToAlbum }: { hit: SearchHit; cache: Map<string, string>; onOpen: () => void; onAddToAlbum: () => void }) {
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
        onAddToAlbum={onAddToAlbum}
      />
    </div>
  );
}

/**
 * Aviso de los resultados por contenido (spec `semantic-search`): cuántas fotos
 * buscadas faltan por analizar y "Analizar", que pone en la cola los proyectos
 * con el disco conectado. El avance se ve en la barra lateral.
 */
function UnanalyzedNotice({ projects }: { projects: SearchResult['unanalyzed'] }) {
  const { t } = useTranslation();
  const [requested, setRequested] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const total = projects.reduce((n, p) => n + p.count, 0);
  const connected = projects.filter((p) => p.connected);
  const away = total - connected.reduce((n, p) => n + p.count, 0);
  if (total === 0) return null;
  const analyze = async () => {
    setError(null);
    try {
      for (const p of connected) await api.analyzeProject(p.project_id);
      setRequested(true);
    } catch (err) {
      setError(String(err));
    }
  };
  return (
    <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: '8px 12px', padding: '8px 12px', marginBottom: '8px', borderRadius: '6px', background: 'var(--lumik-surface-container, #201f1f)', fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '13px', color: 'var(--lumik-on-surface-variant, #c2c6d7)' }}>
      <Icon name="search" size="sm" aria-hidden="true" />
      <span>
        {t('search.unanalyzed.count', { count: total })}
        {away > 0 && ` · ${t('search.unanalyzed.away', { count: away })}`}
        {requested && ` · ${t('search.unanalyzed.requested')}`}
        {error && ` · ${t('search.unanalyzed.error', { error })}`}
      </span>
      {connected.length > 0 && !requested && (
        <Button variant="ghost" size="sm" onClick={analyze}>{t('search.unanalyzed.analyze')}</Button>
      )}
    </div>
  );
}

interface Props {
  /** Consulta de los resultados (para agregarlos todos a un álbum). */
  query: string;
  result: SearchResult | null;
  searching: boolean;
  error: string | null;
  /** Posición a la que volver (al regresar de otra sección). */
  initialScrollTop: number;
  /** Guarda la posición actual (para volver igual desde otra sección). */
  onScrollSave?: (scrollTop: number) => void;
  /** El visor se abre o se cierra (el dashboard pausa sus atajos mientras está abierto). */
  onViewerChange?: (open: boolean) => void;
  /** Vuelve al grid de proyectos. */
  onClose: () => void;
}

/**
 * Resultados de la búsqueda de fotos (spec `photo-search`): agrupados por
 * proyecto y virtualizados. Una foto cuyo disco no está conectado muestra un
 * aviso con su miniatura y el disco que hay que conectar.
 */
export function PhotoSearchResults({ query, result, searching, error, initialScrollTop, onScrollSave, onViewerChange, onClose }: Props) {
  const { t } = useTranslation();
  const [offline, setOffline] = useState<SearchHit | null>(null);
  const [albumTarget, setAlbumTarget] = useState<AlbumTarget | null>(null);
  // Foto abierta en el visor; las flechas recorren los resultados con disco conectado.
  const [viewing, setViewing] = useState<string | null>(null);
  // Cambios hechos en el visor, sobre lo que trajo la búsqueda.
  const [edits, setEdits] = useState<Record<string, PhotoEdits>>({});
  const viewerRefs = useMemo(
    () => (result?.hits ?? []).filter((h) => h.connected).map((h) => ({ projectId: h.project_id, photoId: h.photo_id })),
    [result],
  );
  const setViewer = (photoId: string | null) => {
    setViewing(photoId);
    onViewerChange?.(photoId !== null);
  };
  // Los resultados se refrescaron (cambiaron los discos): el visor se cierra.
  const shownResult = useRef(result);
  useEffect(() => {
    if (shownResult.current === result) return;
    shownResult.current = result;
    if (viewing) setViewer(null);
  }, [result]); // eslint-disable-line react-hooks/exhaustive-deps
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

  const hits = useMemo(
    () => (result?.hits ?? []).map((h) => (edits[h.photo_id] ? { ...h, ...edits[h.photo_id] } : h)),
    [result, edits],
  );
  const groups = useMemo(() => groupByProject(hits), [hits]);
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

  const openHit = (hit: SearchHit) => {
    if (!hit.connected) return setOffline(hit);
    onScrollSave?.(scrollEl?.scrollTop ?? 0);
    setViewer(hit.photo_id);
  };

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
        {result && result.hits.length > 0 && (
          <Button variant="ghost" size="sm" leftIcon={<Icon name="album" size="sm" />} onClick={() => setAlbumTarget({ query, count: result.hits.length })}>
            {t('albums.add.allResults')}
          </Button>
        )}
        <Button variant="ghost" size="sm" leftIcon={<Icon name="x" size="sm" />} onClick={onClose}>
          {t('search.backToProjects')}
        </Button>
      </div>

      {result && <UnanalyzedNotice key={query} projects={result.unanalyzed} />}

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
                    <SearchCard
                      key={hit.photo_id}
                      hit={hit}
                      cache={thumbCache.current}
                      onOpen={() => openHit(hit)}
                      onAddToAlbum={() => setAlbumTarget({ items: [{ kind: 'photo', project_id: hit.project_id, ref_id: hit.photo_id, file_name: hit.file_name, project_name: hit.project_name }] })}
                    />
                  ))}
                </div>
              );
            })}
          </div>
        )}
      </div>

      <AddToAlbumDialog target={albumTarget} onClose={() => setAlbumTarget(null)} />
      {viewing && (
        <CollectionViewer
          refs={viewerRefs}
          startPhotoId={viewing}
          onClose={() => setViewer(null)}
          onPhotoEdited={(photoId, change) => setEdits((all) => ({ ...all, [photoId]: { ...all[photoId], ...change } }))}
        />
      )}

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
