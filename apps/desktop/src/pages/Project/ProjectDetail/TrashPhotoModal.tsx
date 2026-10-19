import { useTranslation } from 'react-i18next';
import { Modal, Button } from '@lumik/ui';

interface TrashPhotoModalProps {
  open: boolean;
  fileName: string;
  onClose: () => void;
  onConfirm: () => void;
  loading?: boolean;
  error?: string | null;
}

const bodyStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '20px',
};

const messageStyles: React.CSSProperties = {
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
  gap: '12px',
};

/** Confirmación de mover una foto a la papelera (`_trash/`): se puede
 *  restaurar, así que no lleva aviso de irreversibilidad. */
export function TrashPhotoModal({
  open,
  fileName,
  onClose,
  onConfirm,
  loading,
  error,
}: TrashPhotoModalProps) {
  const { t } = useTranslation();

  const handleClose = () => {
    if (loading) return;
    onClose();
  };

  return (
    <Modal
      title={t('projectDetail.trashPhoto.title')}
      open={open}
      onClose={handleClose}
      closable
      style={{ width: '460px' }}
    >
      <div style={bodyStyles}>
        {error && <div style={errorStyles}>{error}</div>}
        <p style={messageStyles}>
          {t('projectDetail.trashPhoto.message', { name: fileName })}
        </p>
        <p style={hintStyles}>{t('projectDetail.trashPhoto.hint')}</p>

        <div style={actionsStyles}>
          <Button variant="ghost" type="button" onClick={handleClose} disabled={loading}>
            {t('projectDetail.trashPhoto.cancel')}
          </Button>
          <Button variant="danger" type="button" onClick={onConfirm} disabled={loading}>
            {loading ? t('common.loading') : t('projectDetail.trashPhoto.confirm')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
