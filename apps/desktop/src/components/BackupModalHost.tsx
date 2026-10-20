import { useBackup } from '../lib/BackupContext';
import { useBackupSession } from '../lib/BackupSessionContext';
import { BackupModal } from '../pages/Project/ProjectDetail/BackupModal';
import { BackupNoSessionModal } from './BackupNoSessionModal';

interface BackupModalHostProps {
  /** Abre Ajustes → Respaldo (lo pasa `App`, que controla la navegación). */
  onOpenBackupSettings: () => void;
}

/** Único lugar donde se monta el modal de respaldo: lo abren el proyecto y las
 *  tarjetas de la barra lateral con `openBackupDetail`, desde cualquier pantalla.
 *  Sin sesión, en lugar del modal muestra el aviso que lleva a iniciar sesión. */
export function BackupModalHost({ onOpenBackupSettings }: BackupModalHostProps) {
  const { detail, jobs, closeBackupDetail } = useBackup();
  const { session, loading } = useBackupSession();
  if (!detail || loading) return null;

  // Un job ya en marcha (p. ej. se cerró sesión a mitad) se sigue mostrando.
  if (!session && !jobs[detail.projectId]) {
    return (
      <BackupNoSessionModal
        onClose={closeBackupDetail}
        onGoToSettings={() => {
          closeBackupDetail();
          onOpenBackupSettings();
        }}
      />
    );
  }

  return (
    <BackupModal
      // Un proyecto distinto es un modal nuevo: no arrastra el estado del anterior.
      key={detail.projectId}
      projectId={detail.projectId}
      projectName={detail.projectName}
      onClose={closeBackupDetail}
    />
  );
}
