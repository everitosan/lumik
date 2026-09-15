import { useEffect, useState, useCallback } from 'react';
import type { CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal, Button, Loader, Icon } from '@lumik/ui';
import * as api from '../../../lib/api';
import type { ExportedBackupItem } from '../../../lib/api';

type RowState = 'pending' | 'uploading' | 'done' | 'error';

interface BackupModalProps {
  projectId: string;
  onClose: () => void;
}

const listStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
  maxHeight: '340px',
  overflowY: 'auto',
  margin: '4px 0 16px',
};

const rowStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '12px',
  padding: '8px 10px',
  border: '1px solid var(--lumik-outline-variant, #424654)',
  borderRadius: 'var(--lumik-radius-sm, 4px)',
  fontSize: '13px',
};

const nameStyles: CSSProperties = {
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};

const footerStyles: CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: '12px',
};

export function BackupModal({ projectId, onClose }: BackupModalProps) {
  const { t } = useTranslation();
  const [items, setItems] = useState<ExportedBackupItem[]>([]);
  const [rowState, setRowState] = useState<Record<string, RowState>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [finished, setFinished] = useState(false);

  useEffect(() => {
    api
      .listExportedBackup(projectId)
      .then((list) => {
        setItems(list);
        // Los ya subidos arrancan como 'done'; el resto 'pending'.
        const initial: Record<string, RowState> = {};
        for (const it of list) initial[it.id] = it.uploaded ? 'done' : 'pending';
        setRowState(initial);
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [projectId]);

  const pending = items.filter((i) => rowState[i.id] === 'pending');

  const run = useCallback(async () => {
    setRunning(true);
    setError(null);
    for (const it of items) {
      if (rowState[it.id] === 'done') continue;
      setRowState((s) => ({ ...s, [it.id]: 'uploading' }));
      try {
        await api.backupExportedFile(projectId, it.id, it.filename);
        setRowState((s) => ({ ...s, [it.id]: 'done' }));
      } catch (e) {
        setRowState((s) => ({ ...s, [it.id]: 'error' }));
        setError(String(e));
      }
    }
    setRunning(false);
    setFinished(true);
  }, [items, rowState, projectId]);

  const renderStatus = (state: RowState) => {
    switch (state) {
      case 'done':
        return <Icon name="check" size={16} />;
      case 'uploading':
        return <Loader size={16} />;
      case 'error':
        return <Icon name="x" size={16} />;
      default:
        return <span style={{ color: 'var(--lumik-on-surface-variant, #c2c6d7)' }}>·</span>;
    }
  };

  return (
    <Modal title={t('backup.modal.title')} open onClose={running ? undefined : onClose}>
      {loading ? (
        <div style={{ padding: '24px', display: 'flex', justifyContent: 'center' }}>
          <Loader />
        </div>
      ) : (
        <>
          <p style={{ fontSize: '13px', color: 'var(--lumik-on-surface-variant, #c2c6d7)', marginTop: 0 }}>
            {items.length === 0
              ? t('backup.modal.empty')
              : t('backup.modal.subtitle', { pending: pending.length, total: items.length })}
          </p>

          {items.length > 0 && (
            <div style={listStyles}>
              {items.map((it) => (
                <div key={it.id} style={rowStyles}>
                  <span style={nameStyles}>{it.filename}</span>
                  {renderStatus(rowState[it.id])}
                </div>
              ))}
            </div>
          )}

          {error && (
            <p style={{ fontSize: '12px', color: 'var(--lumik-error, #ffb4ab)' }}>{error}</p>
          )}

          <div style={footerStyles}>
            <Button variant="ghost" onClick={onClose} disabled={running}>
              {finished ? t('backup.modal.close') : t('backup.modal.cancel')}
            </Button>
            {!finished && (
              <Button
                variant="primary"
                onClick={run}
                disabled={running || pending.length === 0}
              >
                {running
                  ? t('backup.uploading')
                  : t('backup.modal.accept', { count: pending.length })}
              </Button>
            )}
          </div>
        </>
      )}
    </Modal>
  );
}
