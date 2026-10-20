import { useTranslation } from 'react-i18next';
import { Modal, Button } from '@lumik/ui';

interface BackupNoSessionModalProps {
  onClose: () => void;
  /** Lleva a Ajustes → Respaldo, donde se inicia sesión o se crea la cuenta. */
  onGoToSettings: () => void;
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

const actionsStyles: React.CSSProperties = {
  display: 'flex',
  justifyContent: 'flex-end',
  gap: '12px',
};

/** Aviso al pulsar "Respaldar" sin sesión de respaldo: en lugar del modal de
 *  respaldo, explica que hace falta una cuenta y lleva a Ajustes → Respaldo. */
export function BackupNoSessionModal({ onClose, onGoToSettings }: BackupNoSessionModalProps) {
  const { t } = useTranslation();
  return (
    <Modal
      title={t('backup.noSession.title')}
      open
      onClose={onClose}
      closable
      style={{ width: '440px' }}
    >
      <div style={bodyStyles}>
        <p style={messageStyles}>{t('backup.noSession.message')}</p>
        <div style={actionsStyles}>
          <Button variant="ghost" type="button" onClick={onClose}>
            {t('backup.noSession.cancel')}
          </Button>
          <Button variant="primary" type="button" onClick={onGoToSettings}>
            {t('backup.noSession.goToSettings')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
