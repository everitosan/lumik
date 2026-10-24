import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Modal, Button, Input } from '@lumik/ui';
import { backupApi, type CatalogProject } from '../api/client';

interface Props {
  project: CatalogProject;
  onClose: () => void;
  /** Tras borrar: el dashboard recarga los proyectos. */
  onDeleted: () => void;
}

/**
 * Confirma y borra el respaldo de un proyecto: su carpeta en cada proveedor y
 * sus fotos del catálogo. Vive solo en el portal (no en la app de escritorio)
 * y pide escribir el nombre del proyecto, para no borrar un respaldo por error.
 */
export function DeleteBackupModal({ project, onClose, onDeleted }: Props) {
  const { t } = useTranslation();
  const [typed, setTyped] = useState('');
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const matches = typed.trim() === project.name.trim();

  const confirm = async () => {
    if (!matches) return;
    setDeleting(true);
    setError(null);
    try {
      await backupApi.deleteBackup(project.id);
      onDeleted();
    } catch (e) {
      setError(e instanceof Error ? e.message : t('auth.errors.generic'));
      setDeleting(false);
    }
  };

  return (
    <Modal title={t('deleteBackup.title')} open onClose={deleting ? undefined : onClose}>
      <div className="delete-body">
        <p>{t('deleteBackup.message', { name: project.name })}</p>
        <p className="delete-warning">{t('deleteBackup.warning')}</p>
        <Input
          label={t('deleteBackup.typeName', { name: project.name })}
          value={typed}
          onChange={(e) => setTyped(e.target.value)}
          disabled={deleting}
          autoComplete="off"
        />
        {error && <p className="auth-error">{error}</p>}
        <div className="upload-footer">
          <Button variant="ghost" onClick={onClose} disabled={deleting}>
            {t('deleteBackup.cancel')}
          </Button>
          <Button variant="danger" onClick={confirm} disabled={!matches || deleting}>
            {deleting ? t('deleteBackup.deleting') : t('deleteBackup.confirm')}
          </Button>
        </div>
      </div>
    </Modal>
  );
}
