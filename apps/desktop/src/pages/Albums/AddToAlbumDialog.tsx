import { useEffect, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Icon, Input, Modal } from '@lumik/ui';
import * as api from '../../lib/api';
import type { AlbumSummary, NewAlbumItem } from '../../lib/types';

/** Qué se agrega: unas fotos concretas, o todo lo que encuentra una búsqueda. */
export type AlbumTarget = { items: NewAlbumItem[] } | { query: string; count: number };

const listStyles: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '4px', maxHeight: '320px', overflowY: 'auto' };
const rowStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: '10px',
  width: '100%',
  minHeight: '44px',
  padding: '0 12px',
  background: 'transparent',
  border: '1px solid transparent',
  borderRadius: '6px',
  color: 'var(--lumik-on-surface, #e5e2e1)',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  textAlign: 'left',
  cursor: 'pointer',
};
const mutedStyles: CSSProperties = { color: 'var(--lumik-on-surface-variant, #c2c6d7)', fontSize: '13px' };

interface Props {
  target: AlbumTarget | null;
  onClose: () => void;
}

/** Elegir (o crear) el álbum al que se agregan fotos (spec `photo-albums`). */
export function AddToAlbumDialog({ target, onClose }: Props) {
  const { t } = useTranslation();
  const [albums, setAlbums] = useState<AlbumSummary[] | null>(null);
  const [newName, setNewName] = useState('');
  const [busy, setBusy] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!target) return;
    setDone(null);
    setError(null);
    setNewName('');
    api.listAlbums().then(setAlbums).catch(() => setAlbums([]));
  }, [target]);

  const total = !target ? 0 : 'items' in target ? target.items.length : target.count;

  const addTo = async (album: AlbumSummary) => {
    if (!target) return;
    setBusy(true);
    setError(null);
    try {
      const added = 'items' in target ? await api.addToAlbum(album.id, target.items) : await api.addSearchToAlbum(album.id, target.query);
      setDone(
        added === total
          ? t('albums.add.done', { count: added, album: album.name })
          : t('albums.add.doneSomeThere', { count: added, there: total - added, album: album.name }),
      );
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  };

  const createAndAdd = async () => {
    if (!newName.trim()) return;
    setBusy(true);
    try {
      const album = await api.createAlbum(newName);
      await addTo(album);
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  };

  return (
    <Modal title={t('albums.add.title', { count: total })} open={target !== null} onClose={onClose} closable style={{ width: '440px' }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '16px', fontFamily: 'var(--lumik-font-primary, Inter)' }}>
        {done ? (
          <>
            <p style={{ margin: 0, fontSize: '14px', lineHeight: 1.5 }}>{done}</p>
            <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
              <Button variant="primary" onClick={onClose}>{t('albums.add.ok')}</Button>
            </div>
          </>
        ) : (
          <>
            <div style={listStyles}>
              {albums === null && <span style={mutedStyles}>{t('common.loading')}</span>}
              {albums?.length === 0 && <span style={mutedStyles}>{t('albums.add.none')}</span>}
              {albums?.map((album) => (
                <button key={album.id} type="button" style={rowStyles} disabled={busy} onClick={() => void addTo(album)}>
                  <Icon name="album" size={18} />
                  <span style={{ flex: 1 }}>{album.name}</span>
                  <span style={mutedStyles}>{t('albums.photos', { count: album.items })}</span>
                </button>
              ))}
            </div>
            <form
              style={{ display: 'flex', gap: '8px', alignItems: 'flex-end' }}
              onSubmit={(e) => { e.preventDefault(); void createAndAdd(); }}
            >
              <div style={{ flex: 1 }}>
                <Input label={t('albums.add.newLabel')} value={newName} placeholder={t('albums.add.newPlaceholder')} onChange={(e) => setNewName(e.target.value)} fullWidth disabled={busy} />
              </div>
              <Button variant="secondary" type="submit" disabled={busy || !newName.trim()}>{t('albums.add.create')}</Button>
            </form>
            {error && <p style={{ margin: 0, fontSize: '13px', color: 'var(--lumik-error, #ffb4ab)' }}>{error}</p>}
          </>
        )}
      </div>
    </Modal>
  );
}
