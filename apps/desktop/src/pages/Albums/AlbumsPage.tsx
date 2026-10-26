import { useEffect, useMemo, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Icon, Input, Modal } from '@lumik/ui';
import * as api from '../../lib/api';
import type { AlbumSummary, AlbumView } from '../../lib/types';
import { useAlbumThumb } from './AlbumThumb';

const pageStyles: CSSProperties = { display: 'flex', flexDirection: 'column', height: '100%', padding: '24px 32px', gap: '24px', overflow: 'hidden' };
const headerStyles: CSSProperties = { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '16px' };
const gridStyles: CSSProperties = { display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))', gap: '24px', paddingBottom: '24px' };
const cardStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  padding: 0,
  background: 'var(--lumik-glass-bg, rgba(32, 31, 31, 0.6))',
  border: '1px solid var(--lumik-glass-border, rgba(66, 70, 84, 0.3))',
  borderRadius: '8px',
  overflow: 'hidden',
  cursor: 'pointer',
  color: 'inherit',
  font: 'inherit',
  textAlign: 'left',
};
const mutedStyles: CSSProperties = { fontSize: '13px', color: 'var(--lumik-on-surface-variant, #c2c6d7)' };

/** Fecha de creación legible, en el idioma de la app. */
function formatCreated(raw: string, language: string): string {
  const d = new Date(raw);
  if (Number.isNaN(d.getTime())) return raw;
  return d.toLocaleDateString(language === 'es' ? 'es-MX' : 'en-US', { day: 'numeric', month: 'long', year: 'numeric' });
}

function AlbumCard({ album, onOpen, onInfo }: { album: AlbumSummary; onOpen: () => void; onInfo: () => void }) {
  const { t, i18n } = useTranslation();
  const cover = useAlbumThumb(album.cover);
  return (
    <div
      style={cardStyles}
      role="button"
      tabIndex={0}
      onClick={onOpen}
      onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onOpen(); } }}
    >
      <div style={{ aspectRatio: '4 / 3', background: 'var(--lumik-surface-container-lowest, #0e0e0e)', display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--lumik-outline, #8c90a0)' }}>
        {cover ? <img src={cover} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover' }} /> : <Icon name="album" size={40} />}
      </div>
      <div style={{ padding: '10px 8px 12px 14px', display: 'flex', alignItems: 'center', gap: '8px', fontFamily: 'var(--lumik-font-primary, Inter)' }}>
        <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: '4px' }}>
          <span style={{ fontSize: '15px', fontWeight: 600, color: 'var(--lumik-on-surface, #e5e2e1)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={album.name}>
            {album.name}
          </span>
          <span style={mutedStyles}>{t('albums.createdOn', { date: formatCreated(album.created_at, i18n.language) })}</span>
        </div>
        <button
          type="button"
          aria-label={t('albums.info.button')}
          title={t('albums.info.button')}
          onClick={(e) => { e.stopPropagation(); onInfo(); }}
          onKeyDown={(e) => e.stopPropagation()}
          style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', width: '36px', height: '36px', flexShrink: 0, background: 'transparent', border: 'none', borderRadius: '6px', color: 'var(--lumik-on-surface-variant, #c2c6d7)', cursor: 'pointer' }}
        >
          <Icon name="info" size={18} />
        </button>
      </div>
    </div>
  );
}

/** Totales del álbum y discos a los que hace referencia, con cuántas fotos hay en cada uno. */
function AlbumInfoDialog({ album, onClose }: { album: AlbumSummary | null; onClose: () => void }) {
  const { t, i18n } = useTranslation();
  const [view, setView] = useState<AlbumView | null>(null);
  useEffect(() => {
    setView(null);
    if (album) api.getAlbum(album.id, 'added').then(setView).catch(() => setView(null));
  }, [album]);

  // Discos de las fotos que siguen en su proyecto, del que tiene más al que tiene menos.
  const disks = useMemo(() => {
    const byDisk = new Map<string, { name: string | null; photos: number; connected: boolean }>();
    for (const e of view?.entries ?? []) {
      if (e.status === 'missing') continue;
      const key = e.device_name ?? '';
      const disk = byDisk.get(key) ?? { name: e.device_name, photos: 0, connected: false };
      disk.photos += 1;
      disk.connected = disk.connected || e.connected;
      byDisk.set(key, disk);
    }
    return [...byDisk.values()].sort((a, b) => b.photos - a.photos);
  }, [view]);

  const rowStyles: CSSProperties = { display: 'flex', alignItems: 'center', gap: '10px', padding: '8px 0', borderBottom: '1px solid var(--lumik-outline-variant, #424654)' };

  return (
    <Modal title={album?.name ?? ''} open={album !== null} onClose={onClose} closable style={{ width: '440px' }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '14px', color: 'var(--lumik-on-surface, #e5e2e1)' }}>
        {album && <span style={mutedStyles}>{t('albums.createdOn', { date: formatCreated(album.created_at, i18n.language) })}</span>}
        {!view ? (
          <span style={mutedStyles}>{t('common.loading')}</span>
        ) : (
          <>
            <div>
              <div style={{ fontSize: '28px', fontWeight: 600 }}>{view.entries.length}</div>
              <div style={mutedStyles}>
                {t('albums.photos', { count: view.entries.length })} · {t('albums.finals', { count: view.finals })} · {t('albums.unedited', { count: view.unedited })}
                {view.missing > 0 && ` · ${t('albums.gone', { count: view.missing })}`}
              </div>
            </div>
            <div>
              <div style={{ ...mutedStyles, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.04em', marginBottom: '4px' }}>{t('albums.info.disks')}</div>
              {disks.length === 0 && <span style={mutedStyles}>{t('albums.info.noDisks')}</span>}
              {disks.map((d) => (
                <div key={d.name ?? ''} style={rowStyles}>
                  <Icon name="drive" size={18} color={d.connected ? 'var(--lumik-primary, #aac0f0)' : 'var(--lumik-outline, #8c90a0)'} />
                  <span style={{ flex: 1 }}>{d.name ?? t('albums.unknownDisk')}</span>
                  <span style={mutedStyles}>{d.connected ? t('albums.info.connected') : t('albums.info.notConnected')}</span>
                  <span style={{ minWidth: '72px', textAlign: 'right' }}>{t('albums.photos', { count: d.photos })}</span>
                </div>
              ))}
            </div>
          </>
        )}
      </div>
    </Modal>
  );
}

/** Lista de álbumes del equipo (spec `photo-albums`). */
export function AlbumsPage({ onOpenAlbum }: { onOpenAlbum: (albumId: string) => void }) {
  const { t } = useTranslation();
  const [albums, setAlbums] = useState<AlbumSummary[] | null>(null);
  const [creating, setCreating] = useState(false);
  const [name, setName] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<AlbumSummary | null>(null);

  useEffect(() => {
    api.listAlbums().then(setAlbums).catch((err) => { setError(String(err)); setAlbums([]); });
  }, []);

  const create = async () => {
    if (!name.trim()) return;
    try {
      const album = await api.createAlbum(name);
      setCreating(false);
      setName('');
      onOpenAlbum(album.id);
    } catch (err) {
      setError(String(err));
    }
  };

  return (
    <div style={pageStyles}>
      <div style={headerStyles}>
        <h1 style={{ margin: 0, fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '24px', fontWeight: 600, color: 'var(--lumik-on-surface, #e5e2e1)' }}>
          {t('albums.title')}
        </h1>
        <Button variant="primary" leftIcon={<Icon name="plus" size="sm" />} onClick={() => setCreating(true)}>
          {t('albums.new')}
        </Button>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', paddingRight: '8px' }}>
        {error && <p style={{ ...mutedStyles, color: 'var(--lumik-error, #ffb4ab)' }}>{error}</p>}
        {albums === null && <p style={mutedStyles}>{t('common.loading')}</p>}
        {albums?.length === 0 && (
          <div style={{ ...mutedStyles, fontSize: '14px', lineHeight: 1.6, maxWidth: '560px' }}>
            <p style={{ marginTop: 0 }}>{t('albums.empty')}</p>
            <p style={{ margin: 0 }}>{t('albums.emptyHint')}</p>
          </div>
        )}
        <div style={gridStyles}>
          {albums?.map((album) => (
            <AlbumCard key={album.id} album={album} onOpen={() => onOpenAlbum(album.id)} onInfo={() => setInfo(album)} />
          ))}
        </div>
      </div>

      <AlbumInfoDialog album={info} onClose={() => setInfo(null)} />

      <Modal title={t('albums.new')} open={creating} onClose={() => setCreating(false)} closable style={{ width: '420px' }}>
        <form style={{ display: 'flex', flexDirection: 'column', gap: '16px' }} onSubmit={(e) => { e.preventDefault(); void create(); }}>
          <Input label={t('albums.nameLabel')} value={name} placeholder={t('albums.add.newPlaceholder')} onChange={(e) => setName(e.target.value)} fullWidth autoFocus />
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: '12px' }}>
            <Button variant="ghost" type="button" onClick={() => setCreating(false)}>{t('albums.cancel')}</Button>
            <Button variant="primary" type="submit" disabled={!name.trim()}>{t('albums.add.create')}</Button>
          </div>
        </form>
      </Modal>
    </div>
  );
}
