import { useEffect, useState, useCallback } from 'react';
import type { CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal, Button, Icon, Select, type SelectOption } from '@lumik/ui';
import * as api from '../../../lib/api';
import type { ExportedBackupItem, ProviderInfo } from '../../../lib/api';

type RowState = 'pending' | 'uploading' | 'done' | 'error';
type Quality = 'original' | 'high' | 'medium' | 'low';
// Subidas simultáneas. Más = más rápido, pero también más CPU de conversión y
// ancho de banda en el server; 6 es un balance razonable.
const CONCURRENCY = 6;

interface BackupModalProps {
  projectId: string;
  onClose: () => void;
}

const listStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
  maxHeight: '300px',
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
const nameStyles: CSSProperties = { overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' };
const fieldStyles: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '8px', marginBottom: '16px' };
const labelStyles: CSSProperties = { fontSize: '13px', fontWeight: 500, color: 'var(--lumik-on-surface-variant, #c2c6d7)' };
const toggleRowStyles: CSSProperties = { display: 'flex', gap: '8px', flexWrap: 'wrap' };
const footerStyles: CSSProperties = { display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '12px' };

/** Ejecuta `worker` sobre `items` con concurrencia limitada. */
async function runPool<T>(items: T[], limit: number, worker: (t: T) => Promise<void>) {
  let idx = 0;
  const runners = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (idx < items.length) {
      const current = items[idx++];
      await worker(current);
    }
  });
  await Promise.all(runners);
}

export function BackupModal({ projectId, onClose }: BackupModalProps) {
  const { t } = useTranslation();
  const [items, setItems] = useState<ExportedBackupItem[]>([]);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [quality, setQuality] = useState<Quality>('high');
  const [rowState, setRowState] = useState<Record<string, RowState>>({});
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const [finished, setFinished] = useState(false);

  useEffect(() => {
    Promise.all([api.listExportedBackup(projectId), api.listBackupProviders()])
      .then(([list, provs]) => {
        setItems(list);
        setProviders(provs);
        setSelected(new Set(provs.map((p) => p.provider))); // todos por defecto
        const initial: Record<string, RowState> = {};
        for (const it of list) initial[it.id] = it.in_progress ? 'uploading' : 'pending';
        setRowState(initial);
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [projectId]);

  const toggleProvider = (p: string) => {
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(p)) next.delete(p);
      else next.add(p);
      return next;
    });
  };

  const pending = items.filter((i) => rowState[i.id] === 'pending');

  const run = useCallback(async () => {
    setRunning(true);
    setError(null);
    const targets = Array.from(selected).join(',');
    const todo = items.filter((i) => rowState[i.id] === 'pending');
    await runPool(todo, CONCURRENCY, async (it) => {
      setRowState((s) => ({ ...s, [it.id]: 'uploading' }));
      try {
        await api.backupExportedFile(projectId, it.id, it.filename, targets, quality);
        setRowState((s) => ({ ...s, [it.id]: 'done' }));
      } catch (e) {
        setRowState((s) => ({ ...s, [it.id]: 'error' }));
        setError(String(e));
      }
    });
    setRunning(false);
    setFinished(true);
  }, [items, rowState, selected, quality, projectId]);

  const renderStatus = (state: RowState) => {
    switch (state) {
      case 'done':
        return <Icon name="check" size={16} />;
      case 'uploading':
        return <span className="lumik-spinner" />;
      case 'error':
        return <Icon name="x" size={16} />;
      default:
        return <span style={{ color: 'var(--lumik-on-surface-variant, #c2c6d7)' }}>·</span>;
    }
  };

  const canUpload = !running && pending.length > 0 && selected.size > 0;

  return (
    <Modal title={t('backup.modal.title')} open onClose={running ? undefined : onClose}>
      {loading ? (
        <div style={{ padding: '32px', display: 'flex', justifyContent: 'center' }}>
          <span className="lumik-spinner lg" />
        </div>
      ) : (
        <>
          {/* Proveedores destino */}
          <div style={fieldStyles}>
            <span style={labelStyles}>{t('backup.modal.providers')}</span>
            {providers.length === 0 ? (
              <span style={{ fontSize: '13px', color: 'var(--lumik-error, #ffb4ab)' }}>
                {t('backup.modal.noProviders')}
              </span>
            ) : (
              <div style={toggleRowStyles}>
                {providers.map((p) => (
                  <Button
                    key={p.provider}
                    variant={selected.has(p.provider) ? 'primary' : 'secondary'}
                    size="sm"
                    onClick={() => toggleProvider(p.provider)}
                    disabled={running}
                  >
                    {t(`providers.types.${p.provider}`, { defaultValue: p.provider })}
                  </Button>
                ))}
              </div>
            )}
          </div>

          {/* Calidad */}
          <div style={fieldStyles}>
            <Select
              label={t('backup.modal.quality')}
              value={quality}
              onChange={(v) => setQuality(v as Quality)}
              disabled={running}
              fullWidth
              options={
                [
                  { value: 'original', label: t('backup.modal.qOriginal') },
                  { value: 'high', label: t('backup.modal.qHigh') },
                  { value: 'medium', label: t('backup.modal.qMedium') },
                  { value: 'low', label: t('backup.modal.qLow') },
                ] as SelectOption[]
              }
            />
          </div>

          {/* Lista de pendientes */}
          <p style={{ fontSize: '13px', color: 'var(--lumik-on-surface-variant, #c2c6d7)', marginTop: 0 }}>
            {items.length === 0
              ? t('backup.modal.empty')
              : t('backup.modal.subtitle', { count: items.length })}
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

          {error && <p style={{ fontSize: '12px', color: 'var(--lumik-error, #ffb4ab)' }}>{error}</p>}

          <div style={footerStyles}>
            <Button variant="ghost" onClick={onClose} disabled={running}>
              {finished ? t('backup.modal.close') : t('backup.modal.cancel')}
            </Button>
            {!finished && (
              <Button variant="primary" onClick={run} disabled={!canUpload}>
                {running ? t('backup.uploading') : t('backup.modal.accept', { count: pending.length })}
              </Button>
            )}
          </div>
        </>
      )}
    </Modal>
  );
}
