import { useEffect, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal } from '@lumik/ui';
import * as api from '../../lib/api';
import type { NewAlbumItem, ReplaceCandidate } from '../../lib/types';
import { useAlbumThumb } from './AlbumThumb';

const gridStyles: CSSProperties = { display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(120px, 1fr))', gap: '10px' };
const tileStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
  padding: '4px',
  background: 'transparent',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: '6px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '11px',
  cursor: 'pointer',
  textAlign: 'left',
};
const sectionStyles: CSSProperties = { margin: '8px 0', fontFamily: 'var(--lumik-font-primary, Inter)', fontSize: '13px', fontWeight: 600, color: 'var(--lumik-on-surface, #e5e2e1)' };

function Tile({ candidate, onPick }: { candidate: ReplaceCandidate; onPick: () => void }) {
  const url = useAlbumThumb(candidate.thumb);
  return (
    <button type="button" style={tileStyles} onClick={onPick} title={candidate.item.file_name}>
      <div style={{ aspectRatio: '4 / 3', background: 'var(--lumik-surface-container-lowest, #0e0e0e)', borderRadius: '4px', overflow: 'hidden' }}>
        {url && <img src={url} alt="" style={{ width: '100%', height: '100%', objectFit: 'cover' }} />}
      </div>
      <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{candidate.item.file_name}</span>
    </button>
  );
}

interface Props {
  /** Proyecto del elemento que se reemplaza; `null` = cerrado. */
  projectId: string | null;
  onPick: (item: NewAlbumItem) => void;
  onClose: () => void;
}

/** "Reemplazar por…": fotos y exportados del mismo proyecto (spec `photo-albums`). */
export function ReplaceDialog({ projectId, onPick, onClose }: Props) {
  const { t } = useTranslation();
  const [candidates, setCandidates] = useState<ReplaceCandidate[] | null>(null);
  useEffect(() => {
    setCandidates(null);
    if (projectId) api.getReplaceCandidates(projectId).then(setCandidates).catch(() => setCandidates([]));
  }, [projectId]);

  const exports = candidates?.filter((c) => c.item.kind === 'export') ?? [];
  const photos = candidates?.filter((c) => c.item.kind === 'photo') ?? [];

  return (
    <Modal title={t('albums.replace.title')} open={projectId !== null} onClose={onClose} closable style={{ width: 'min(760px, 92vw)' }}>
      <div style={{ maxHeight: '60vh', overflowY: 'auto' }}>
        {candidates === null && <p style={{ ...sectionStyles, fontWeight: 400 }}>{t('common.loading')}</p>}
        {exports.length > 0 && (
          <>
            <h3 style={sectionStyles}>{t('albums.replace.exports')}</h3>
            <div style={gridStyles}>{exports.map((c) => <Tile key={c.item.ref_id} candidate={c} onPick={() => onPick(c.item)} />)}</div>
          </>
        )}
        {photos.length > 0 && (
          <>
            <h3 style={sectionStyles}>{t('albums.replace.photos')}</h3>
            <div style={gridStyles}>{photos.map((c) => <Tile key={c.item.ref_id} candidate={c} onPick={() => onPick(c.item)} />)}</div>
          </>
        )}
      </div>
    </Modal>
  );
}
