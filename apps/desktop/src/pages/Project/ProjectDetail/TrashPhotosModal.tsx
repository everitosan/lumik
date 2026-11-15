import { useTranslation } from 'react-i18next';
import { Modal, Button } from '@lumik/ui';
import type { BatchFailure } from '../../../lib/types';
import { describeBatchReason } from '../../../lib/batch';

interface TrashPhotosModalProps {
  open: boolean;
  count: number;
  onClose: () => void;
  onConfirm: () => void;
  loading?: boolean;
  error?: string | null;
  /** Fotos que no se pudieron mover en el último intento. */
  failed?: BatchFailure[];
}

const bodyStyles: React.CSSProperties = { display: 'flex', flexDirection: 'column', gap: '16px' };
const messageStyles: React.CSSProperties = { margin: 0, fontSize: '14px', lineHeight: 1.5, color: 'var(--lumik-on-surface, #e5e2e1)' };
const hintStyles: React.CSSProperties = { margin: 0, fontSize: '13px', lineHeight: 1.5, color: 'var(--lumik-on-surface-variant, #c2c6d7)' };
const errorStyles: React.CSSProperties = {
  padding: '12px 16px',
  backgroundColor: 'var(--lumik-error-container, #93000a)',
  color: 'var(--lumik-on-error-container, #ffdad6)',
  borderRadius: '8px',
  fontSize: '13px',
  lineHeight: 1.5,
};
const actionsStyles: React.CSSProperties = { display: 'flex', justifyContent: 'flex-end', gap: '12px' };

/** Confirmación de mandar varias fotos marcadas a la papelera (spec photo-trash).
 *  Si alguna falla, se queda abierto con la lista de las que no se movieron. */
export function TrashPhotosModal({ open, count, onClose, onConfirm, loading, error, failed }: TrashPhotosModalProps) {
  const { t } = useTranslation();
  const close = () => !loading && onClose();
  const reported = failed && failed.length > 0;
  return (
    <Modal title={t('projectDetail.marked.trashTitle')} open={open} onClose={close} closable style={{ width: '480px' }}>
      <div style={bodyStyles}>
        {error && <div style={errorStyles}>{error}</div>}
        {reported ? (
          <div style={errorStyles}>
            <p style={{ margin: '0 0 8px' }}>{t('batch.failed', { count: failed.length })}</p>
            <ul style={{ margin: 0, paddingLeft: '18px' }}>
              {failed.map((f) => (
                <li key={f.id}>
                  {f.name}: {describeBatchReason(t, f.reason)}
                </li>
              ))}
            </ul>
          </div>
        ) : (
          <>
            <p style={messageStyles}>{t('projectDetail.marked.trashMessage', { count })}</p>
            <p style={hintStyles}>{t('projectDetail.trashPhoto.hint')}</p>
          </>
        )}
        <div style={actionsStyles}>
          <Button variant="ghost" type="button" onClick={close} disabled={loading}>
            {reported ? t('albums.add.ok') : t('projectDetail.trashPhoto.cancel')}
          </Button>
          {!reported && (
            <Button variant="danger" type="button" onClick={onConfirm} disabled={loading}>
              {loading ? t('common.loading') : t('projectDetail.trashPhoto.confirm')}
            </Button>
          )}
        </div>
      </div>
    </Modal>
  );
}
