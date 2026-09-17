import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
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

const MAX_ATTEMPTS = 3;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Reintenta `fn` con backoff exponencial ante fallos transitorios. */
async function withRetry<T>(fn: () => Promise<T>): Promise<T> {
  let lastErr: unknown;
  for (let attempt = 0; attempt < MAX_ATTEMPTS; attempt++) {
    try {
      return await fn();
    } catch (e) {
      lastErr = e;
      if (attempt < MAX_ATTEMPTS - 1) await sleep(600 * 2 ** attempt); // 0.6s, 1.2s
    }
  }
  throw lastErr;
}

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

      // Persiste la intención (cola offline-first): sobrevive al cierre de la app.
      void api.setPendingBackup(projectId, targets, quality);

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

      let errors = 0;
      const uploadOne = async (f: { id: string; filename: string }) => {
        setStatus(f.id, 'uploading');
        try {
          // Reintentos con backoff ante fallos transitorios (red, etc.).
          await withRetry(() =>
            api.backupExportedFile(projectId, f.id, f.filename, targets, quality)
          );
          setStatus(f.id, 'done');
        } catch {
          errors += 1;
          setStatus(f.id, 'error');
        }
      };

      const finish = async () => {
        // Solo si todo subió bien: fija el estado del proyecto (respaldado/editando)
        // y saca el proyecto de la cola. Si hubo errores, se queda encolado para
        // reanudar en el próximo arranque.
        if (errors === 0 && files.length > 0) {
          try {
            await api.finalizeBackup(projectId, targets);
          } catch {
            /* best-effort */
          }
          void api.clearPendingBackup(projectId);
        }
        setJobs((prev) => {
          const job = prev[projectId];
          return job ? { ...prev, [projectId]: { ...job, running: false } } : prev;
        });
      };

      void (async () => {
        // 1) Crea el directorio remoto UNA vez (secuencial). Mega permite carpetas
        //    con el mismo nombre; crearlo en paralelo generaba `_exported` duplicados.
        try {
          await api.prepareBackup(projectId, targets);
        } catch {
          // Si falla el prepare, subir el primero en solitario también crea el
          // directorio antes del pool (fallback).
          if (files[0]) await uploadOne(files[0]);
          await runPool(files.slice(1), CONCURRENCY, uploadOne);
          await finish();
          return;
        }
        // 2) Directorio ya creado → subir todo en paralelo.
        await runPool(files, CONCURRENCY, uploadOne);
        await finish();
      })();
    },
    []
  );

  // Reanuda al arrancar: por cada proyecto en la cola, recalcula pendientes
  // (disco _exported − catálogo) y relanza. Si el disco no está montado o hay
  // fallo de red, se salta y queda encolado para el próximo arranque.
  const resumed = useRef(false);
  useEffect(() => {
    if (resumed.current) return;
    resumed.current = true;
    void (async () => {
      let pending: api.PendingBackup[];
      try {
        pending = await api.getPendingBackups();
      } catch {
        return;
      }
      for (const req of pending) {
        try {
          const items = await api.listExportedBackup(req.project_id);
          const files = items
            .filter((i) => !i.in_progress)
            .map((i) => ({ id: i.id, filename: i.filename }));
          if (files.length === 0) {
            void api.clearPendingBackup(req.project_id); // nada pendiente
          } else {
            startBackup(req.project_id, files, req.targets ?? '', req.quality ?? 'high');
          }
        } catch {
          /* proyecto no montado / offline: se queda en la cola */
        }
      }
    })();
  }, [startBackup]);

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
