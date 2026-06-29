import { useTranslation } from 'react-i18next';
import { Modal, Button } from '@lumik/ui';

interface DeleteProjectModalProps {
  open: boolean;
  projectName: string;
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
  fontSize: '14px',
  lineHeight: 1.5,
  color: 'var(--lumik-on-surface, #e5e2e1)',
};

const warningStyles: React.CSSProperties = {
  padding: '12px 16px',
  backgroundColor: 'var(--lumik-error-container, #93000a)',
  color: 'var(--lumik-on-error-container, #ffdad6)',
  borderRadius: '8px',
  fontSize: '13px',
  lineHeight: 1.5,
};

const errorStyles: React.CSSProperties = {
  ...warningStyles,
  backgroundColor: 'var(--lumik-error-container, #93000a)',
};

const actionsStyles: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'flex-end',
  gap: '12px',
};

export function DeleteProjectModal({
  open,
  projectName,
  onClose,
  onConfirm,
  loading,
  error,
}: DeleteProjectModalProps) {
  const { t } = useTranslation();

  const handleClose = () => {
    if (loading) return;
    onClose();
  };

  return (
    <Modal
      title={t('project.delete.title')}
      open={open}
      onClose={handleClose}
      closable
      style={{ width: '460px' }}
    >
      <div style={bodyStyles}>
        {error && <div style={errorStyles}>{error}</div>}

        <p style={messageStyles}>
          {t('project.delete.message', { name: projectName })}
        </p>
        <div style={warningStyles}>{t('project.delete.warning')}</div>

        <div style={actionsStyles}>
          <Button variant="ghost" type="button" onClick={handleClose} disabled={loading}>
            {t('project.create.buttons.cancel')}
          </Button>
          <Button variant="danger" type="button" onClick={onConfirm} disabled={loading}>
            {loading ? t('common.loading') : t('project.delete.confirm')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
