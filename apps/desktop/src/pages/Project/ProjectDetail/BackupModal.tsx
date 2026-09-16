import { useEffect, useState } from 'react';
import type { CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal, Button, Icon, Select, type SelectOption } from '@lumik/ui';
import * as api from '../../../lib/api';
import type { ExportedBackupItem, ProviderInfo } from '../../../lib/api';
import { useBackup, type FileStatus } from '../../../lib/BackupContext';

type Quality = 'original' | 'high' | 'medium' | 'low';

interface BackupModalProps {
  projectId: string;
  onClose: () => void;
}

const listStyles: CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '6px',
  maxHeight: '260px',
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
const mutedStyles: CSSProperties = { fontSize: '13px', color: 'var(--lumik-on-surface-variant, #c2c6d7)', marginTop: 0 };

export function BackupModal({ projectId, onClose }: BackupModalProps) {
  const { t } = useTranslation();
  const { jobs, startBackup } = useBackup();
  const job = jobs[projectId];

  // Si al abrir ya hay un job corriendo, mostramos el progreso directo.
  const [viewingJob, setViewingJob] = useState<boolean>(job?.running ?? false);

  const [items, setItems] = useState<ExportedBackupItem[]>([]);
  const [providers, setProviders] = useState<ProviderInfo[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [quality, setQuality] = useState<Quality>('high');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Modo configuración: cargar proveedores + pendientes (no subidas).
  useEffect(() => {
    if (viewingJob) {
      setLoading(false);
      return;
    }
    setLoading(true);
    Promise.all([api.listExportedBackup(projectId), api.listBackupProviders()])
      .then(([list, provs]) => {
        setItems(list);
        setProviders(provs);
        setSelected(new Set(provs.map((p) => p.provider)));
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [projectId, viewingJob]);

  const toggleProvider = (p: string) =>
    setSelected((s) => {
      const next = new Set(s);
      if (next.has(p)) next.delete(p);
      else next.add(p);
      return next;
    });

  const pendingItems = items.filter((i) => !i.in_progress);

  const startUpload = () => {
    const targets = Array.from(selected).join(',');
    startBackup(
      projectId,
      pendingItems.map((i) => ({ id: i.id, filename: i.filename })),
      targets,
      quality
    );
    setViewingJob(true);
  };

  const renderStatus = (state: FileStatus) => {
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

  // ---- Vista PROGRESO (job en curso o recién terminado) ----
  if (viewingJob && job) {
    const done = job.files.filter((f) => f.status === 'done').length;
    const failed = job.files.filter((f) => f.status === 'error').length;
    return (
      <Modal title={t('backup.modal.title')} open onClose={onClose}>
        <p style={mutedStyles}>
          {job.running
            ? t('backup.modal.progress', { done, total: job.files.length })
            : t('backup.modal.finished', { done, total: job.files.length })}
        </p>
        {job.running && (
          <p style={{ ...mutedStyles, fontSize: '12px' }}>{t('backup.modal.canClose')}</p>
        )}
        <div style={listStyles}>
          {job.files.map((f) => (
            <div key={f.id} style={rowStyles}>
              <span style={nameStyles}>{f.filename}</span>
              {renderStatus(f.status)}
            </div>
          ))}
        </div>
        {failed > 0 && !job.running && (
          <p style={{ fontSize: '12px', color: 'var(--lumik-error, #ffb4ab)' }}>
            {t('backup.modal.someFailed', { failed })}
          </p>
        )}
        <div style={footerStyles}>
          <span />
          <Button variant="primary" onClick={onClose}>
            {job.running ? t('backup.modal.closeKeep') : t('backup.modal.close')}
          </Button>
        </div>
      </Modal>
    );
  }

  // ---- Vista CONFIGURACIÓN ----
  const canUpload = pendingItems.length > 0 && selected.size > 0;
  return (
    <Modal title={t('backup.modal.title')} open onClose={onClose}>
      {loading ? (
        <div style={{ padding: '32px', display: 'flex', justifyContent: 'center' }}>
          <span className="lumik-spinner lg" />
        </div>
      ) : (
        <>
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
                  >
                    {t(`providers.types.${p.provider}`, { defaultValue: p.provider })}
                  </Button>
                ))}
              </div>
            )}
          </div>

          <div style={fieldStyles}>
            <Select
              label={t('backup.modal.quality')}
              value={quality}
              onChange={(v) => setQuality(v as Quality)}
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

          <p style={mutedStyles}>
            {pendingItems.length === 0
              ? t('backup.modal.empty')
              : t('backup.modal.subtitle', { count: pendingItems.length })}
          </p>
          {pendingItems.length > 0 && (
            <div style={listStyles}>
              {pendingItems.map((it) => (
                <div key={it.id} style={rowStyles}>
                  <span style={nameStyles}>{it.filename}</span>
                  {renderStatus('pending')}
                </div>
              ))}
            </div>
          )}

          {error && <p style={{ fontSize: '12px', color: 'var(--lumik-error, #ffb4ab)' }}>{error}</p>}

          <div style={footerStyles}>
            <Button variant="ghost" onClick={onClose}>
              {t('backup.modal.cancel')}
            </Button>
            <Button variant="primary" onClick={startUpload} disabled={!canUpload}>
              {t('backup.modal.accept', { count: pendingItems.length })}
            </Button>
          </div>
        </>
      )}
    </Modal>
  );
}
