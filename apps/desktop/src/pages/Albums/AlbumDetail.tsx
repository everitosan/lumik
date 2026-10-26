import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { useVirtualizer } from '@tanstack/react-virtual';
import { Button, Icon, Input, Modal, PhotoCard, PHOTO_CARD_HEADER_HEIGHT, PHOTO_CARD_FOOTER_HEIGHT, Select, photoFormat, type ColorLabel } from '@lumik/ui';
import * as api from '../../lib/api';
import { useDevicesKey } from '../../lib/hooks';
import type { AlbumEntry, AlbumOrder, AlbumView, NewAlbumItem } from '../../lib/types';
import { useAlbumThumb } from './AlbumThumb';
import { AddToAlbumDialog, type AlbumTarget } from './AddToAlbumDialog';
import { ReplaceDialog } from './ReplaceDialog';
import { CollectionViewer, type PhotoEdits } from '../Collection/CollectionViewer';

const CARD_MIN_WIDTH = 200;
const COL_GAP = 16;
const ROW_GAP = 16;
const CARD_H = PHOTO_CARD_HEADER_HEIGHT + PHOTO_CARD_FOOTER_HEIGHT;

const pageStyles: CSSProperties = { display: 'flex', flexDirection: 'column', height: '100%', padding: '24px 32px', gap: '16px', overflow: 'hidden' };
const mutedStyles: CSSProperties = { fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '13px', color: 'var(--lumik-on-surface-variant, #c2c6d7)' };
const noticeStyles: CSSProperties = {
  ...mutedStyles,
  padding: '10px 14px',
  border: '1px solid var(--lumik-secondary, #e9c349)',
  borderRadius: '8px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  lineHeight: 1.5,
};
const chipBase: CSSProperties = {
  position: 'absolute',
  zIndex: 1,
  left: '8px',
  top: `${PHOTO_CARD_HEADER_HEIGHT + 8}px`,
  padding: '2px 8px',
  borderRadius: '999px',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '11px',
  fontWeight: 600,
  pointerEvents: 'none',
};

function StatusChip({ entry }: { entry: AlbumEntry }) {
  const { t } = useTranslation();
  const style: CSSProperties =
    entry.status === 'final'
      ? { ...chipBase, background: 'var(--lumik-secondary, #e9c349)', color: '#131313' }
      : entry.status === 'unedited'
        ? { ...chipBase, background: 'rgba(14, 14, 14, 0.8)', color: 'var(--lumik-on-surface, #e5e2e1)' }
        : { ...chipBase, background: 'var(--lumik-error-container, #93000a)', color: 'var(--lumik-on-error-container, #ffdad6)' };
  return <span style={style}>{t(`albums.status.${entry.status}`)}</span>;
}

interface CardProps {
  entry: AlbumEntry;
  onOpen: () => void;
  onAddToAlbum: () => void;
  onReveal: () => void;
  onReplace: () => void;
  onRemove: () => void;
}

function AlbumCard({ entry, onOpen, onAddToAlbum, onReveal, onReplace, onRemove }: CardProps) {
  const { t } = useTranslation();
  const url = useAlbumThumb(entry.thumb);
  const gone = entry.status === 'missing';
  return (
    <div style={{ position: 'relative', opacity: gone ? 0.55 : entry.connected ? 1 : 0.8 }}>
      <StatusChip entry={entry} />
      <PhotoCard
        filename={entry.file_name}
        thumbnailUrl={url ?? undefined}
        stars={entry.stars}
        culled={entry.culled}
        captureDate={entry.capture_date ?? undefined}
        colorLabels={entry.colors.filter((c) => c >= 1 && c <= 5) as ColorLabel[]}
        format={entry.is_pair && entry.status === 'unedited' ? 'RAW+JPG' : photoFormat(entry.file_name)}
        onClick={onOpen}
        onAddToAlbum={gone ? undefined : onAddToAlbum}
        onRevealInFolder={gone ? undefined : onReveal}
        extraMenuItems={[
          { icon: 'edit', label: t('albums.item.replace'), onSelect: onReplace },
          { icon: 'x', label: t('albums.item.remove'), onSelect: onRemove, danger: true },
        ]}
      />
    </div>
  );
}

/** Un elemento como algo que se puede agregar a otro álbum. */
function asNewItem(entry: AlbumEntry): NewAlbumItem {
  return { kind: entry.kind, project_id: entry.project_id, ref_id: entry.ref_id, file_name: entry.file_name, project_name: entry.project_name };
}

interface Props {
  albumId: string;
  onBack: () => void;
}

/** Un álbum: curación de fotos de varios proyectos y discos (spec `photo-albums`). */
export function AlbumDetail({ albumId, onBack }: Props) {
  const { t } = useTranslation();
  const [order, setOrder] = useState<AlbumOrder>('added');
  const [view, setView] = useState<AlbumView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editingName, setEditingName] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [addTarget, setAddTarget] = useState<AlbumTarget | null>(null);
  const [replacing, setReplacing] = useState<AlbumEntry | null>(null);
  // Foto abierta en el visor; las flechas recorren los elementos que se pueden abrir.
  const [viewing, setViewing] = useState<string | null>(null);
  // Cambios hechos en el visor, aplicados sobre lo que dice el índice (que se
  // pone al día unos segundos después de cada escritura).
  const [edits, setEdits] = useState<Record<string, PhotoEdits>>({});
  const onPhotoEdited = useCallback(
    (photoId: string, change: PhotoEdits) => setEdits((all) => ({ ...all, [photoId]: { ...all[photoId], ...change } })),
    [],
  );

  const load = useCallback(() => {
    api.getAlbum(albumId, order).then(setView).catch((err) => setError(String(err)));
  }, [albumId, order]);
  useEffect(() => { load(); }, [load]);

  // Al conectar o expulsar un disco: qué se puede abrir y qué discos faltan.
  // Con el visor abierto se cierra, como al expulsar el disco de un proyecto abierto.
  const devicesKey = useDevicesKey();
  const lastDevicesKey = useRef<string | null>(null);
  useEffect(() => {
    if (devicesKey === null) return;
    const changed = lastDevicesKey.current !== null && lastDevicesKey.current !== devicesKey;
    lastDevicesKey.current = devicesKey;
    if (!changed) return;
    setViewing(null);
    load();
  }, [devicesKey, load]);

  // ── Grid virtualizado ─────────────────────────────────────────────────────
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
  const rows = useMemo(() => {
    const entries = (view?.entries ?? []).map((e) => (e.photo_id && edits[e.photo_id] ? { ...e, ...edits[e.photo_id] } : e));
    const out: AlbumEntry[][] = [];
    for (let i = 0; i < entries.length; i += cols) out.push(entries.slice(i, i + cols));
    return out;
  }, [view, edits, cols]);
  const virtualizer = useVirtualizer({ count: rows.length, getScrollElement: () => scrollEl, estimateSize: () => rowHeight, overscan: 3 });
  useEffect(() => { virtualizer.measure(); }, [rowHeight, rows.length]); // eslint-disable-line react-hooks/exhaustive-deps
  const viewerRefs = useMemo(
    () =>
      (view?.entries ?? [])
        .filter((e) => e.connected && e.status !== 'missing' && e.photo_id)
        .map((e) => ({ projectId: e.project_id, photoId: e.photo_id as string })),
    [view],
  );

  const diskMessage = (device: string | null) =>
    device ? t('albums.connectDisk', { device }) : t('albums.connectDiskUnknown');

  const open = (entry: AlbumEntry) => {
    if (entry.status === 'missing') return setNotice(t('albums.item.gone'));
    if (!entry.connected) return setNotice(diskMessage(entry.device_name));
    if (!entry.photo_id) return setNotice(t('albums.item.exportOnly'));
    setViewing(entry.photo_id);
  };

  const reveal = async (entry: AlbumEntry) => {
    try {
      await api.revealAlbumItem(entry.item_id);
    } catch (err) {
      const disk = api.missingDiskOf(err);
      setNotice(disk !== null ? diskMessage(disk || null) : String(err));
    }
  };

  const saveName = async () => {
    if (editingName === null) return;
    const name = editingName.trim();
    setEditingName(null);
    if (!name || name === view?.name) return;
    await api.renameAlbum(albumId, name).catch((err) => setError(String(err)));
    load();
  };

  const remove = async () => {
    await api.deleteAlbum(albumId);
    onBack();
  };

  return (
    <div style={pageStyles}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '12px', flexWrap: 'wrap' }}>
        <Button variant="ghost" size="sm" leftIcon={<Icon name="chevron-left" size="sm" />} onClick={onBack}>{t('albums.title')}</Button>
        {editingName !== null ? (
          <form style={{ minWidth: '280px' }} onSubmit={(e) => { e.preventDefault(); void saveName(); }}>
            <Input value={editingName} onChange={(e) => setEditingName(e.target.value)} onBlur={() => void saveName()} autoFocus fullWidth aria-label={t('albums.nameLabel')} />
          </form>
        ) : (
          <button
            type="button"
            title={t('albums.rename')}
            onClick={() => setEditingName(view?.name ?? '')}
            style={{ background: 'transparent', border: 'none', padding: 0, cursor: 'text', fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '22px', fontWeight: 600, color: 'var(--lumik-on-surface, #e5e2e1)' }}
          >
            {view?.name ?? ''}
          </button>
        )}
        {view && (
          <span style={mutedStyles}>
            {t('albums.photos', { count: view.entries.length })} · {t('albums.finals', { count: view.finals })} · {t('albums.unedited', { count: view.unedited })}
            {view.missing > 0 && ` · ${t('albums.gone', { count: view.missing })}`}
          </span>
        )}
        <div style={{ marginLeft: 'auto', display: 'flex', gap: '12px', alignItems: 'center' }}>
          <div style={{ width: '200px' }}>
            <Select
              value={order}
              onChange={(v) => setOrder(v as AlbumOrder)}
              options={[
                { value: 'added', label: t('albums.order.added') },
                { value: 'captured', label: t('albums.order.captured') },
              ]}
              fullWidth
            />
          </div>
          <Button variant="ghost" size="sm" leftIcon={<Icon name="trash" size="sm" />} onClick={() => setConfirmDelete(true)}>{t('albums.delete')}</Button>
        </div>
      </div>

      {view && view.missing_disks.length > 0 && (
        <div style={noticeStyles} role="status">
          {t('albums.missingDisks', { count: view.missing_disks.length })}{' '}
          {view.missing_disks
            .map((d) => `${d.device_name ?? t('albums.unknownDisk')} (${t('albums.photos', { count: d.photos })})`)
            .join(', ')}
        </div>
      )}

      <div ref={setScrollEl} style={{ flex: 1, overflowY: 'auto', paddingRight: '8px' }}>
        {error && <p style={{ ...mutedStyles, color: 'var(--lumik-error, #ffb4ab)' }}>{error}</p>}
        {view && view.entries.length === 0 && <p style={{ ...mutedStyles, fontSize: '14px' }}>{t('albums.emptyAlbum')}</p>}
        {rows.length > 0 && (
          <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
            {virtualizer.getVirtualItems().map((v) => (
              <div
                key={v.key}
                style={{ position: 'absolute', top: 0, left: 0, width: '100%', transform: `translateY(${v.start}px)`, display: 'grid', gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))`, gap: `${COL_GAP}px`, paddingBottom: `${ROW_GAP}px` }}
              >
                {rows[v.index]?.map((entry) => (
                  <AlbumCard
                    key={entry.item_id}
                    entry={entry}
                    onOpen={() => open(entry)}
                    onAddToAlbum={() => setAddTarget({ items: [asNewItem(entry)] })}
                    onReveal={() => void reveal(entry)}
                    onReplace={() => setReplacing(entry)}
                    onRemove={() => void api.removeAlbumItem(entry.item_id).then(load)}
                  />
                ))}
              </div>
            ))}
          </div>
        )}
      </div>

      <Modal title={t('albums.noticeTitle')} open={notice !== null} onClose={() => setNotice(null)} closable style={{ width: '420px' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          <p style={{ ...mutedStyles, margin: 0, fontSize: '14px', lineHeight: 1.5, color: 'var(--lumik-on-surface, #e5e2e1)' }}>{notice}</p>
          <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
            <Button variant="primary" onClick={() => setNotice(null)}>{t('albums.add.ok')}</Button>
          </div>
        </div>
      </Modal>

      <Modal title={t('albums.deleteTitle')} open={confirmDelete} onClose={() => setConfirmDelete(false)} closable style={{ width: '440px' }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '16px' }}>
          <p style={{ ...mutedStyles, margin: 0, fontSize: '14px', lineHeight: 1.5, color: 'var(--lumik-on-surface, #e5e2e1)' }}>
            {t('albums.deleteMessage', { name: view?.name ?? '' })}
          </p>
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '12px' }}>
            <Button variant="ghost" onClick={() => setConfirmDelete(false)}>{t('albums.cancel')}</Button>
            <Button variant="danger" onClick={() => void remove()}>{t('albums.delete')}</Button>
          </div>
        </div>
      </Modal>

      <AddToAlbumDialog target={addTarget} onClose={() => { setAddTarget(null); load(); }} />
      {viewing && (
        <CollectionViewer
          refs={viewerRefs}
          startPhotoId={viewing}
          onPhotoEdited={onPhotoEdited}
          onClose={() => { setViewing(null); load(); }}
        />
      )}
      <ReplaceDialog
        projectId={replacing?.project_id ?? null}
        onClose={() => setReplacing(null)}
        onPick={(item) => {
          const target = replacing;
          setReplacing(null);
          if (target) void api.replaceAlbumItem(target.item_id, item).then(load);
        }}
      />
    </div>
  );
}
