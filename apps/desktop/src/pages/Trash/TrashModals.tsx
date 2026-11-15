// Modales de la sección "Papelera" (spec trash-bin): eliminar definitivamente,
// vaciar la papelera de un disco y elegir otro nombre cuando el lugar al que
// vuelve un elemento está ocupado.
import { useEffect, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Input, Modal } from '@lumik/ui';
import type { BatchFailure, RestoreCheck } from '../../lib/types';
import { describeBatchReason } from '../../lib/batch';

const bodyStyles: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '16px' };

const messageStyles: CSSProperties = {
  margin: 0,
  fontSize: '14px',
  lineHeight: 1.5,
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const hintStyles: CSSProperties = {
  margin: 0,
  fontSize: '13px',
  lineHeight: 1.5,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const warningStyles: CSSProperties = {
  padding: '12px 16px',
  backgroundColor: 'var(--lumik-error-container, #93000a)',
  color: 'var(--lumik-on-error-container, #ffdad6)',
  borderRadius: '8px',
  fontSize: '13px',
  lineHeight: 1.5,
};

const actionsStyles: CSSProperties = { display: 'flex', justifyContent: 'flex-end', gap: '12px' };

function ErrorBox({ error }: { error?: string | null }) {
  return error ? <div style={warningStyles}>{error}</div> : null;
}

interface PurgeModalProps {
  open: boolean;
  /** Proyecto completo o una foto suelta (con el nombre de su proyecto). */
  target: { kind: 'project'; name: string } | { kind: 'photo'; name: string; project: string } | null;
  onClose: () => void;
  onConfirm: () => void;
  loading?: boolean;
  error?: string | null;
}

/** Confirmación de eliminar definitivamente: lleva el aviso rojo de irreversibilidad. */
export function PurgeModal({ open, target, onClose, onConfirm, loading, error }: PurgeModalProps) {
  const { t } = useTranslation();
  const close = () => !loading && onClose();
  return (
    <Modal title={t('trash.purge.title')} open={open} onClose={close} closable style={{ width: '460px' }}>
      <div style={bodyStyles}>
        <ErrorBox error={error} />
        <p style={messageStyles}>
          {target?.kind === 'project'
            ? t('trash.purge.projectMessage', { name: target.name })
            : target
              ? t('trash.purge.photoMessage', { name: target.name, project: target.project })
              : null}
        </p>
        <div style={warningStyles}>{t('trash.purge.warning')}</div>
        <p style={hintStyles}>{t('trash.purge.backupNote')}</p>
        <div style={actionsStyles}>
          <Button variant="ghost" type="button" onClick={close} disabled={loading}>
            {t('trash.purge.cancel')}
          </Button>
          <Button variant="danger" type="button" onClick={onConfirm} disabled={loading}>
            {loading ? t('common.loading') : t('trash.purge.confirm')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}

interface EmptyTrashModalProps {
  open: boolean;
  disk: string;
  projects: number;
  photos: number;
  onClose: () => void;
  onConfirm: () => void;
  loading?: boolean;
  error?: string | null;
  /** Elementos que no se pudieron eliminar en el último intento. */
  failed?: { name: string; reason: string }[];
}

export function EmptyTrashModal({ open, disk, projects, photos, onClose, onConfirm, loading, error, failed }: EmptyTrashModalProps) {
  const { t } = useTranslation();
  const close = () => !loading && onClose();
  const nothingLeft = projects + photos === 0;
  return (
    <Modal title={t('trash.empty.title', { disk })} open={open} onClose={close} closable style={{ width: '480px' }}>
      <div style={bodyStyles}>
        <ErrorBox error={error} />
        {failed && failed.length > 0 && (
          <div style={warningStyles}>
            <p style={{ margin: '0 0 8px' }}>{t('trash.empty.failed', { count: failed.length })}</p>
            <ul style={{ margin: 0, paddingLeft: '18px' }}>
              {failed.map((f, i) => (
                <li key={i}>
                  {f.name}: {f.reason}
                </li>
              ))}
            </ul>
          </div>
        )}
        {!nothingLeft && (
          <>
            <p style={messageStyles}>
              {t('trash.empty.message', {
                disk,
                projects: t('trash.empty.projects', { count: projects }),
                photos: t('trash.empty.photos', { count: photos }),
              })}
            </p>
            <div style={warningStyles}>{t('trash.purge.warning')}</div>
            <p style={hintStyles}>{t('trash.purge.backupNote')}</p>
          </>
        )}
        <div style={actionsStyles}>
          <Button variant="ghost" type="button" onClick={close} disabled={loading}>
            {t('trash.purge.cancel')}
          </Button>
          {!nothingLeft && (
            <Button variant="danger" type="button" onClick={onConfirm} disabled={loading}>
              {loading ? t('common.loading') : t('trash.empty.confirm')}
            </Button>
          )}
        </div>
      </div>
    </Modal>
  );
}

interface RestoreCollisionModalProps {
  open: boolean;
  kind: 'project' | 'photo';
  /** Nombre del elemento que se restaura. */
  name: string;
  /** Lo que ocupa el lugar (carpeta o archivo). */
  occupied: string;
  /** Valor inicial del campo. */
  initialValue: string;
  /** Dice si el destino con ese nombre está libre (solo lee). */
  check: (value: string) => Promise<RestoreCheck>;
  onClose: () => void;
  onConfirm: (value: string) => void;
  loading?: boolean;
  error?: string | null;
}

/** Pide otro nombre cuando el lugar al que vuelve un elemento está ocupado. No
 *  deja confirmar un nombre vacío ni uno que también esté ocupado. */
export function RestoreCollisionModal({
  open,
  kind,
  name,
  occupied,
  initialValue,
  check,
  onClose,
  onConfirm,
  loading,
  error,
}: RestoreCollisionModalProps) {
  const { t } = useTranslation();
  const [value, setValue] = useState(initialValue);
  const [status, setStatus] = useState<{ value: string; check: RestoreCheck } | null>(null);
  const [checkError, setCheckError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setValue(initialValue);
      setStatus(null);
      setCheckError(null);
    }
  }, [open, initialValue]);

  const trimmed = value.trim();
  useEffect(() => {
    if (!open || !trimmed) return;
    let cancelled = false;
    const timer = setTimeout(() => {
      check(trimmed)
        .then((c) => {
          if (!cancelled) {
            setStatus({ value: trimmed, check: c });
            setCheckError(null);
          }
        })
        .catch((err) => !cancelled && setCheckError(String(err)));
    }, 200);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [open, trimmed, check]);

  const current = status && status.value === trimmed ? status.check : null;
  const canConfirm = !!trimmed && current?.free === true && !loading;
  const close = () => !loading && onClose();

  return (
    <Modal title={t('trash.collision.title')} open={open} onClose={close} closable style={{ width: '480px' }}>
      <form
        style={bodyStyles}
        onSubmit={(e) => {
          e.preventDefault();
          if (canConfirm) onConfirm(trimmed);
        }}
      >
        <ErrorBox error={error ?? checkError} />
        <p style={messageStyles}>
          {kind === 'project'
            ? t('trash.collision.projectMessage', { name, target: occupied })
            : t('trash.collision.photoMessage', { name, target: occupied })}
        </p>
        <Input
          label={kind === 'project' ? t('trash.collision.projectLabel') : t('trash.collision.photoLabel')}
          value={value}
          onChange={(e) => setValue((e.target as HTMLInputElement).value)}
          autoFocus
          disabled={loading}
        />
        {!trimmed && <p style={hintStyles}>{t('trash.collision.emptyName')}</p>}
        {current && !current.free && <p style={{ ...hintStyles, color: 'var(--lumik-error, #ffb4ab)' }}>{t('trash.collision.taken', { target: current.target })}</p>}
        <div style={actionsStyles}>
          <Button variant="ghost" type="button" onClick={close} disabled={loading}>
            {t('trash.collision.cancel')}
          </Button>
          <Button variant="primary" type="submit" disabled={!canConfirm}>
            {loading ? t('common.loading') : t('trash.collision.confirm')}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

interface BatchPurgeModalProps {
  open: boolean;
  projects: number;
  photos: number;
  onClose: () => void;
  onConfirm: () => void;
  loading?: boolean;
  error?: string | null;
}

/** Confirmación de eliminar definitivamente los elementos marcados (spec trash-bin). */
export function BatchPurgeModal({ open, projects, photos, onClose, onConfirm, loading, error }: BatchPurgeModalProps) {
  const { t } = useTranslation();
  const close = () => !loading && onClose();
  return (
    <Modal title={t('trash.purge.title')} open={open} onClose={close} closable style={{ width: '480px' }}>
      <div style={bodyStyles}>
        <ErrorBox error={error} />
        <p style={messageStyles}>
          {t('trash.marked.purgeMessage', {
            projects: t('trash.empty.projects', { count: projects }),
            photos: t('trash.empty.photos', { count: photos }),
          })}
        </p>
        <div style={warningStyles}>{t('trash.purge.warning')}</div>
        <p style={hintStyles}>{t('trash.purge.backupNote')}</p>
        <div style={actionsStyles}>
          <Button variant="ghost" type="button" onClick={close} disabled={loading}>
            {t('trash.purge.cancel')}
          </Button>
          <Button variant="danger" type="button" onClick={onConfirm} disabled={loading}>
            {loading ? t('common.loading') : t('trash.purge.confirm')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}

/** Lo que no se pudo hacer en un lote, con su motivo. */
export function BatchReportModal({ failed, onClose }: { failed: BatchFailure[] | null; onClose: () => void }) {
  const { t } = useTranslation();
  return (
    <Modal title={t('trash.marked.reportTitle')} open={failed !== null && failed.length > 0} onClose={onClose} closable style={{ width: '480px' }}>
      <div style={bodyStyles}>
        <div style={warningStyles}>
          <p style={{ margin: '0 0 8px' }}>{t('batch.failed', { count: failed?.length ?? 0 })}</p>
          <ul style={{ margin: 0, paddingLeft: '18px' }}>
            {failed?.map((f) => (
              <li key={f.id}>
                {f.name}: {describeBatchReason(t, f.reason)}
              </li>
            ))}
          </ul>
        </div>
        <p style={hintStyles}>{t('trash.marked.reportHint')}</p>
        <div style={actionsStyles}>
          <Button variant="primary" type="button" onClick={onClose}>
            {t('albums.add.ok')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
