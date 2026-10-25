import { useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Modal } from '@lumik/ui';
import type { ProjectLockHolder } from '../lib/types';

/** Hora de inicio del candado: solo la hora si es de hoy; si no, fecha y hora. */
function formatSince(since: string, language: string): string {
  const d = new Date(since);
  if (Number.isNaN(d.getTime())) return since;
  const locale = language === 'es' ? 'es-MX' : 'en-US';
  const today = new Date().toDateString() === d.toDateString();
  return today
    ? d.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' })
    : d.toLocaleString(locale, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
}

/** Texto del aviso: suave si el disco está al día, fuerte si quedaron cambios fuera. */
export function useLockMessage(holder: ProjectLockHolder): string {
  const { t, i18n } = useTranslation();
  const params = { name: holder.name, time: formatSince(holder.since, i18n.language) };
  return holder.pending ? t('projectLock.notice.pending', params) : t('projectLock.notice.upToDate', params);
}

const bannerStyles = (pending: boolean): CSSProperties => ({
  display: 'flex',
  alignItems: 'center',
  flexWrap: 'wrap',
  gap: '12px 16px',
  margin: '12px 32px 0',
  padding: '12px 16px',
  borderRadius: '8px',
  backgroundColor: pending
    ? 'var(--lumik-error-container, #93000a)'
    : 'var(--lumik-surface-container-high, #2a2a2a)',
  color: pending
    ? 'var(--lumik-on-error-container, #ffdad6)'
    : 'var(--lumik-on-surface, #e5e2e1)',
  border: pending ? 'none' : '1px solid var(--lumik-secondary, #e9c349)',
  fontFamily: 'var(--lumik-font-primary, Inter)',
  fontSize: '14px',
  lineHeight: 1.5,
});

const errorStyles: CSSProperties = {
  flexBasis: '100%',
  margin: 0,
  fontSize: '13px',
  color: 'var(--lumik-error, #ffb4ab)',
};

interface BannerProps {
  holder: ProjectLockHolder;
  onTakeControl: () => Promise<void>;
}

/**
 * Aviso del proyecto en solo lectura (spec `project-lock`): quién lo tiene,
 * desde cuándo, si el disco está al día, y "Tomar el control".
 */
export function ProjectLockBanner({ holder, onTakeControl }: BannerProps) {
  const { t } = useTranslation();
  const message = useLockMessage(holder);
  const [taking, setTaking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const take = async () => {
    setTaking(true);
    setError(null);
    try {
      await onTakeControl();
    } catch (err) {
      setError(t('projectLock.takeError', { error: String(err) }));
    } finally {
      setTaking(false);
    }
  };

  return (
    <div role="status" style={bannerStyles(holder.pending)}>
      <span style={{ flex: '1 1 320px' }}>{message}</span>
      <Button variant={holder.pending ? 'danger' : 'secondary'} onClick={take} disabled={taking}>
        {taking ? t('projectLock.taking') : t('projectLock.takeControl')}
      </Button>
      {error && <p style={errorStyles}>{error}</p>}
    </div>
  );
}

const bodyStyles: CSSProperties = { display: 'flex', flexDirection: 'column', gap: '16px' };
const actionsStyles: CSSProperties = { display: 'flex', justifyContent: 'flex-end', gap: '12px' };

interface DialogProps {
  holder: ProjectLockHolder | null;
  onClose: () => void;
  /** Toma el control y sigue con la acción que se pidió. */
  onTakeControl: () => Promise<void>;
}

/** Confirma tomar el control antes de editar, archivar o eliminar desde el dashboard. */
export function TakeControlDialog({ holder, onClose, onTakeControl }: DialogProps) {
  const { t } = useTranslation();
  return (
    <Modal title={t('projectLock.dialogTitle')} open={holder !== null} onClose={onClose} closable style={{ width: '480px' }}>
      {holder && <TakeControlBody holder={holder} onClose={onClose} onTakeControl={onTakeControl} />}
    </Modal>
  );
}

function TakeControlBody({ holder, onClose, onTakeControl }: DialogProps & { holder: ProjectLockHolder }) {
  const { t } = useTranslation();
  const message = useLockMessage(holder);
  const [taking, setTaking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const take = async () => {
    setTaking(true);
    setError(null);
    try {
      await onTakeControl();
    } catch (err) {
      setError(t('projectLock.takeError', { error: String(err) }));
      setTaking(false);
    }
  };

  return (
    <div style={bodyStyles}>
      <p style={{ margin: 0, fontSize: '14px', lineHeight: 1.5 }}>{message}</p>
      {error && <p style={errorStyles}>{error}</p>}
      <div style={actionsStyles}>
        <Button variant="ghost" onClick={onClose} disabled={taking}>
          {t('projectLock.cancel')}
        </Button>
        <Button variant={holder.pending ? 'danger' : 'primary'} onClick={take} disabled={taking}>
          {taking ? t('projectLock.taking') : t('projectLock.takeControl')}
        </Button>
      </div>
    </div>
  );
}
