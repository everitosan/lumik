import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal, Button, Select, type SelectOption } from '@lumik/ui';
import {
  providersApi,
  backupApi,
  catalogApi,
  exportedId,
  type Remote,
  type CatalogProject,
} from '../api/client';

type Quality = 'original' | 'high' | 'medium' | 'low';
type RowState = 'pending' | 'uploading' | 'done' | 'error';
const CONCURRENCY = 6;

async function runPool<T>(items: T[], limit: number, worker: (t: T) => Promise<void>) {
  let idx = 0;
  const runners = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (idx < items.length) await worker(items[idx++]);
  });
  await Promise.all(runners);
}

interface Props {
  project: CatalogProject;
  onClose: () => void;
}

export function ProjectBackupModal({ project, onClose }: Props) {
  const { t } = useTranslation();
  const [providers, setProviders] = useState<Remote[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [quality, setQuality] = useState<Quality>('high');
  const [files, setFiles] = useState<File[]>([]);
  const [state, setState] = useState<Record<string, RowState>>({});
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [uploadedIds, setUploadedIds] = useState<Set<string>>(new Set());
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    providersApi
      .list()
      .then((p) => {
        setProviders(p);
        setSelected(new Set(p.map((x) => x.provider)));
      })
      .catch(() => setProviders([]));
    // Fotos ya subidas: para saltar duplicados por nombre (mismo uuid5).
    catalogApi
      .listPhotos(project.id)
      .then((photos) =>
        setUploadedIds(
          new Set(photos.filter((ph) => ph.backup_status === 'uploaded').map((ph) => ph.id))
        )
      )
      .catch(() => setUploadedIds(new Set()));
  }, [project.id]);

  const isUploaded = (file: File) => uploadedIds.has(exportedId(project.id, file.name));

  const addFiles = (list: FileList | null) => {
    if (!list) return;
    const incoming = Array.from(list);
    setFiles((prev) => {
      const names = new Set(prev.map((f) => f.name));
      return [...prev, ...incoming.filter((f) => !names.has(f.name))];
    });
  };

  const toggleProvider = (p: string) =>
    setSelected((s) => {
      const next = new Set(s);
      next.has(p) ? next.delete(p) : next.add(p);
      return next;
    });

  const upload = async () => {
    setRunning(true);
    setError(null);
    const targets = Array.from(selected).join(',');
    // Solo las que no están ya en el remote (evita duplicados por nombre).
    const toUpload = files.filter((f) => !isUploaded(f));
    if (toUpload.length === 0) {
      setRunning(false);
      return;
    }
    try {
      await backupApi.prepare(project.id, targets);
    } catch (e) {
      setError(String(e));
      setRunning(false);
      return;
    }
    await runPool(toUpload, CONCURRENCY, async (file) => {
      const id = exportedId(project.id, file.name);
      setState((s) => ({ ...s, [file.name]: 'uploading' }));
      try {
        await backupApi.uploadFile(project.id, id, file, targets, quality);
        setState((s) => ({ ...s, [file.name]: 'done' }));
      } catch (e) {
        setState((s) => ({ ...s, [file.name]: 'error' }));
        setError(String(e));
      }
    });
    setRunning(false);
  };

  const statusIcon = (st: RowState | undefined) => {
    if (st === 'done') return '✓';
    if (st === 'uploading') return <span className="lumik-spinner" />;
    if (st === 'error') return '✗';
    return '·';
  };

  const pendingCount = files.filter((f) => !isUploaded(f)).length;
  const canUpload = !running && pendingCount > 0 && selected.size > 0;

  return (
    <Modal title={t('upload.title', { project: project.name })} open onClose={running ? undefined : onClose}>
      <div className="upload-field">
        <span className="upload-label">{t('providers.title')}</span>
        {providers.length === 0 ? (
          <span className="auth-error">{t('upload.noProviders')}</span>
        ) : (
          <div className="upload-toggles">
            {providers.map((p) => (
              <Button
                key={p.id}
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

      <div className="upload-field">
        <Select
          label={t('upload.quality')}
          value={quality}
          onChange={(v) => setQuality(v as Quality)}
          fullWidth
          options={
            [
              { value: 'original', label: t('upload.qOriginal') },
              { value: 'high', label: t('upload.qHigh') },
              { value: 'medium', label: t('upload.qMedium') },
              { value: 'low', label: t('upload.qLow') },
            ] as SelectOption[]
          }
        />
      </div>

      <input
        ref={inputRef}
        type="file"
        accept="image/*"
        multiple
        style={{ display: 'none' }}
        onChange={(e) => addFiles(e.target.files)}
      />

      {files.length === 0 ? (
        <div className="upload-dropzone" onClick={() => inputRef.current?.click()}>
          {t('upload.pick')}
        </div>
      ) : (
        <div className="upload-list">
          {files.map((f) => (
            <div key={f.name} className="upload-row">
              <span className="upload-name">{f.name}</span>
              <span>{isUploaded(f) ? '✓' : statusIcon(state[f.name])}</span>
            </div>
          ))}
        </div>
      )}

      {error && <p className="auth-error">{error}</p>}

      <div className="upload-footer">
        <Button variant="ghost" onClick={onClose} disabled={running}>
          {t('upload.close')}
        </Button>
        <div style={{ display: 'flex', gap: '8px' }}>
          {files.length > 0 && !running && (
            <Button variant="secondary" onClick={() => inputRef.current?.click()}>
              {t('upload.addMore')}
            </Button>
          )}
          <Button variant="primary" onClick={upload} disabled={!canUpload}>
            {running ? t('upload.uploading') : t('upload.submit', { count: pendingCount })}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
