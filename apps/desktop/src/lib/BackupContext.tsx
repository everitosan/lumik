import {
  createContext,
  useCallback,
  useContext,
  useState,
  type ReactNode,
} from 'react';
import * as api from './api';

export type FileStatus = 'pending' | 'uploading' | 'done' | 'error';

export interface JobFile {
  id: string;
  filename: string;
  status: FileStatus;
}

export interface BackupJob {
  running: boolean;
  files: JobFile[];
}

interface BackupContextValue {
  /** Jobs de respaldo en curso/terminados, por projectId. */
  jobs: Record<string, BackupJob>;
  /** Lanza el respaldo de `files` para un proyecto. Corre en background. */
  startBackup: (
    projectId: string,
    files: { id: string; filename: string }[],
    targets: string,
    quality: string
  ) => void;
}

const BackupContext = createContext<BackupContextValue | null>(null);

// Subidas simultáneas. Más = más rápido, pero más CPU/ancho de banda en el server.
const CONCURRENCY = 6;

/** Ejecuta `worker` sobre `items` con concurrencia limitada. */
async function runPool<T>(items: T[], limit: number, worker: (t: T) => Promise<void>) {
  let idx = 0;
  const runners = Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (idx < items.length) {
      const current = items[idx++];
      await worker(current);
    }
  });
  await Promise.all(runners);
}

/**
 * Orquesta los respaldos a nivel de app: el proceso vive aquí (no en el modal),
 * así que el usuario puede cerrar el modal y seguir usando la app mientras sube.
 * Al reabrir el modal, este lee el job y muestra el progreso en vivo.
 */
export function BackupProvider({ children }: { children: ReactNode }) {
  const [jobs, setJobs] = useState<Record<string, BackupJob>>({});

  const startBackup = useCallback(
    (
      projectId: string,
      files: { id: string; filename: string }[],
      targets: string,
      quality: string
    ) => {
      let alreadyRunning = false;
      setJobs((prev) => {
        if (prev[projectId]?.running) {
          alreadyRunning = true;
          return prev;
        }
        return {
          ...prev,
          [projectId]: {
            running: true,
            files: files.map((f) => ({ ...f, status: 'pending' as FileStatus })),
          },
        };
      });
      if (alreadyRunning || files.length === 0) return;

      const setStatus = (id: string, status: FileStatus) =>
        setJobs((prev) => {
          const job = prev[projectId];
          if (!job) return prev;
          return {
            ...prev,
            [projectId]: {
              ...job,
              files: job.files.map((f) => (f.id === id ? { ...f, status } : f)),
            },
          };
        });

      const uploadOne = async (f: { id: string; filename: string }) => {
        setStatus(f.id, 'uploading');
        try {
          await api.backupExportedFile(projectId, f.id, f.filename, targets, quality);
          setStatus(f.id, 'done');
        } catch {
          setStatus(f.id, 'error');
        }
      };

      void (async () => {
        // Sube el primero en solitario para crear el directorio remoto UNA vez.
        // Mega permite carpetas con el mismo nombre; si el dir se crea en paralelo
        // (6 procesos rclone a la vez) se generan `_exported` duplicados. Tras el
        // primero, el resto reutiliza la carpeta existente.
        const [first, ...rest] = files;
        if (first) await uploadOne(first);
        await runPool(rest, CONCURRENCY, uploadOne);
        setJobs((prev) => {
          const job = prev[projectId];
          if (!job) return prev;
          return { ...prev, [projectId]: { ...job, running: false } };
        });
      })();
    },
    []
  );

  return (
    <BackupContext.Provider value={{ jobs, startBackup }}>
      {children}
    </BackupContext.Provider>
  );
}

export function useBackup(): BackupContextValue {
  const ctx = useContext(BackupContext);
  if (!ctx) throw new Error('useBackup debe usarse dentro de <BackupProvider>');
  return ctx;
}
