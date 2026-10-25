import { useCallback, useRef, useState, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal, Button } from '@lumik/ui';
import { planProjectDeliveryChange, setProjectDeliveryMode } from '../lib/api';
import type { DeliveryMode, DeliveryPlan } from '../lib/types';

const bodyStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '12px',
};

const lineStyles: React.CSSProperties = {
  margin: 0,
  fontSize: '14px',
  lineHeight: 1.5,
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const hintStyles: React.CSSProperties = {
  margin: 0,
  fontSize: '13px',
  lineHeight: 1.5,
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

const errorStyles: React.CSSProperties = {
  padding: '12px 16px',
  backgroundColor: 'var(--lumik-error-container, #93000a)',
  color: 'var(--lumik-on-error-container, #ffdad6)',
  borderRadius: '8px',
  fontSize: '13px',
  lineHeight: 1.5,
};

const actionsStyles: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'flex-end',
  flexWrap: 'wrap',
  gap: '12px',
  marginTop: '8px',
};

interface Pending {
  mode: DeliveryMode;
  plan: DeliveryPlan;
  resolve: (saved: boolean) => void;
}

/**
 * Cambio de "Al aprobar, entregar directamente" con su aviso (spec
 * `camera-jpeg-delivery`): si el cambio afecta a fotos ya aprobadas, pregunta
 * si se aplica también a ellas; si no, guarda sin preguntar. `requestChange`
 * resuelve `true` si se guardó y `false` si se canceló. `dialog` va en el JSX
 * de quien lo usa (Editar proyecto, paso de ajustes del import).
 */
export function useDeliveryChange(projectId: string): {
  requestChange: (mode: DeliveryMode) => Promise<boolean>;
  dialog: ReactNode;
} {
  const { t } = useTranslation();
  const [pending, setPending] = useState<Pending | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pendingRef = useRef<Pending | null>(null);

  const requestChange = useCallback(
    async (mode: DeliveryMode) => {
      const plan = await planProjectDeliveryChange(projectId, mode);
      if (plan.deliver + plan.return_back + plan.edits_kept === 0) {
        await setProjectDeliveryMode(projectId, mode, false);
        return true;
      }
      return new Promise<boolean>((resolve) => {
        const next = { mode, plan, resolve };
        pendingRef.current = next;
        setError(null);
        setPending(next);
      });
    },
    [projectId],
  );

  const finish = (saved: boolean) => {
    pendingRef.current?.resolve(saved);
    pendingRef.current = null;
    setPending(null);
    setError(null);
  };

  const save = async (applyToApproved: boolean) => {
    if (!pending) return;
    setBusy(true);
    try {
      const outcome = await setProjectDeliveryMode(projectId, pending.mode, applyToApproved);
      if (outcome && outcome.failed.length > 0) {
        // La opción ya quedó guardada: se avisa de lo que no se pudo mover.
        pendingRef.current?.resolve(true);
        pendingRef.current = null;
        setError(t('project.delivery.change.failed', { count: outcome.failed.length, reason: outcome.failed[0][1] }));
        return;
      }
      finish(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  };

  const plan = pending?.plan;
  const dialog = (
    <Modal
      title={t('project.delivery.change.title')}
      open={pending !== null}
      onClose={() => !busy && finish(pendingRef.current === null)}
      closable
      style={{ width: '520px' }}
    >
      {plan && (
        <div style={bodyStyles}>
          {plan.deliver > 0 && <p style={lineStyles}>{t('project.delivery.change.deliver', { count: plan.deliver })}</p>}
          {plan.return_back > 0 && <p style={lineStyles}>{t('project.delivery.change.return', { count: plan.return_back })}</p>}
          {plan.edits_kept > 0 && <p style={hintStyles}>{t('project.delivery.change.editsKept', { count: plan.edits_kept })}</p>}
          {plan.in_cloud > 0 && <p style={hintStyles}>{t('project.delivery.change.inCloud', { count: plan.in_cloud })}</p>}
          {error ? (
            <>
              <div style={errorStyles}>{error}</div>
              <div style={actionsStyles}>
                <Button variant="primary" onClick={() => finish(true)}>
                  {t('project.delivery.change.close')}
                </Button>
              </div>
            </>
          ) : (
            <>
              <p style={lineStyles}>{t('project.delivery.change.question')}</p>
              <div style={actionsStyles}>
                <Button variant="ghost" onClick={() => finish(false)} disabled={busy}>
                  {t('project.delivery.change.cancel')}
                </Button>
                <Button variant="secondary" onClick={() => save(false)} disabled={busy}>
                  {t('project.delivery.change.onlyOption')}
                </Button>
                <Button variant="primary" onClick={() => save(true)} disabled={busy}>
                  {t('project.delivery.change.apply')}
                </Button>
              </div>
            </>
          )}
        </div>
      )}
    </Modal>
  );

  return { requestChange, dialog };
}
