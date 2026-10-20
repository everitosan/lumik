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
import { useBackupSession } from './BackupSessionContext';

export type FileStatus = 'pending' | 'uploading' | 'done' | 'error';

export interface JobFile {
  id: string;
  filename: string;
  status: FileStatus;
}

export interface BackupJob {
  projectName: string;
  /** Proveedores y calidad con que se lanzó: los reutiliza "Reintentar". */
  targets: string;
  quality: string;
  running: boolean;
  files: JobFile[];
}

/** Resultado de `retryBackup`: por qué no se relanzó, si no se pudo. */
export type RetryResult = { ok: true } | { ok: false; reason: 'not_mounted' | 'busy' | 'nothing' };

interface BackupContextValue {
  /** Jobs en curso o terminados con errores, por projectId. Los que terminan
   *  bien se retiran. */
  jobs: Record<string, BackupJob>;
  /** Lanza el respaldo de `files` para un proyecto. Corre en background. */
  startBackup: (
    projectId: string,
    projectName: string,
    files: { id: string; filename: string }[],
    targets: string,
    quality: string
  ) => void;
  /** Vuelve a subir solo los archivos fallidos de un job, con sus proveedores
   *  y calidad. Comprueba antes que el disco del proyecto esté montado. */
  retryBackup: (projectId: string) => Promise<RetryResult>;
  /** Quita el job de la lista sin tocar la cola persistente: se reanudará en el
   *  próximo arranque. */
  dismissBackup: (projectId: string) => void;
  /** Sube con cada respaldo lanzado por el usuario (no con los reanudados al
   *  arrancar). La barra lateral lo observa para expandir su sección. */
  userLaunches: number;
  /** Proyecto cuyo modal de respaldo está abierto (lo pinta `BackupModalHost`). */
  detail: { projectId: string; projectName: string } | null;
  openBackupDetail: (projectId: string, projectName: string) => void;
  closeBackupDetail: () => void;
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
 * El modal y la barra lateral leen los jobs y muestran el progreso en vivo.
 */
export function BackupProvider({ children }: { children: ReactNode }) {
  const [jobs, setJobs] = useState<Record<string, BackupJob>>({});
  const [userLaunches, setUserLaunches] = useState(0);
  const [detail, setDetail] = useState<BackupContextValue['detail']>(null);
  const { session } = useBackupSession();
  // Copia síncrona de `jobs` para decidir en los callbacks sin esperar al render.
  const jobsRef = useRef(jobs);
  jobsRef.current = jobs;

  /** Sube `files` de un job ya registrado en `jobs` y cierra el job al final. */
  const run = useCallback(
    (projectId: string, files: { id: string; filename: string }[], targets: string, quality: string) => {
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
        // Solo si todo subió bien: fija el estado del proyecto (respaldado/editando),
        // saca el proyecto de la cola y retira el job de la lista. Si hubo errores,
        // el job se queda visible y el proyecto encolado para reanudar al arrancar.
        if (errors === 0) {
          try {
            await api.finalizeBackup(projectId, targets);
          } catch {
            /* best-effort */
          }
          void api.clearPendingBackup(projectId);
          setJobs((prev) => {
            if (!prev[projectId]) return prev;
            const { [projectId]: _done, ...rest } = prev;
            return rest;
          });
          return;
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

  /** Registra un job nuevo y lo lanza. Devuelve false si ya había uno corriendo. */
  const launch = useCallback(
    (
      projectId: string,
      projectName: string,
      files: { id: string; filename: string }[],
      targets: string,
      quality: string
    ): boolean => {
      if (jobsRef.current[projectId]?.running || files.length === 0) return false;
      const job: BackupJob = {
        projectName,
        targets,
        quality,
        running: true,
        files: files.map((f) => ({ ...f, status: 'pending' as FileStatus })),
      };
      jobsRef.current = { ...jobsRef.current, [projectId]: job };
      setJobs((prev) => ({ ...prev, [projectId]: job }));
      run(projectId, files, targets, quality);
      return true;
    },
    [run]
  );

  const startBackup = useCallback<BackupContextValue['startBackup']>(
    (projectId, projectName, files, targets, quality) => {
      if (launch(projectId, projectName, files, targets, quality)) {
        setUserLaunches((n) => n + 1);
      }
    },
    [launch]
  );

  const retryBackup = useCallback<BackupContextValue['retryBackup']>(
    async (projectId) => {
      const job = jobsRef.current[projectId];
      if (!job) return { ok: false, reason: 'nothing' };
      if (job.running) return { ok: false, reason: 'busy' };
      const failed = job.files.filter((f) => f.status === 'error');
      if (failed.length === 0) return { ok: false, reason: 'nothing' };

      // Los archivos se leen de `_exported/`: sin el disco no hay nada que subir.
      try {
        if (!(await api.getProject(projectId))) return { ok: false, reason: 'not_mounted' };
      } catch {
        return { ok: false, reason: 'not_mounted' };
      }

      // Se reutiliza el job: los subidos se conservan y solo los fallidos vuelven
      // a `pending`, así el avance sigue desde donde estaba (21/23 → 23/23).
      const failedIds = new Set(failed.map((f) => f.id));
      const next: BackupJob = {
        ...job,
        running: true,
        files: job.files.map((f) => (failedIds.has(f.id) ? { ...f, status: 'pending' } : f)),
      };
      jobsRef.current = { ...jobsRef.current, [projectId]: next };
      setJobs((prev) => ({ ...prev, [projectId]: next }));
      run(
        projectId,
        failed.map((f) => ({ id: f.id, filename: f.filename })),
        job.targets,
        job.quality
      );
      return { ok: true };
    },
    [run]
  );

  const dismissBackup = useCallback((projectId: string) => {
    setJobs((prev) => {
      if (!prev[projectId] || prev[projectId].running) return prev;
      const { [projectId]: _dismissed, ...rest } = prev;
      return rest;
    });
  }, []);

  const openBackupDetail = useCallback((projectId: string, projectName: string) => {
    setDetail({ projectId, projectName });
  }, []);
  const closeBackupDetail = useCallback(() => setDetail(null), []);

  // Reanuda la cola la primera vez que hay sesión en esta ejecución (al arrancar
  // o al iniciar sesión): sin sesión las subidas fallarían con 401. Por cada
  // proyecto en la cola, recalcula pendientes (disco _exported − catálogo) y
  // relanza. Si el disco no está montado o hay fallo de red, se salta y queda
  // encolado para la próxima vez.
  const resumed = useRef(false);
  const hasSession = session != null;
  useEffect(() => {
    if (resumed.current || !hasSession) return;
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
            // La cola solo guarda el id: el nombre sale del proyecto (ya montado,
            // porque listar sus pendientes acaba de funcionar).
            const project = await api.getProject(req.project_id).catch(() => null);
            launch(
              req.project_id,
              project?.name ?? req.project_id,
              files,
              req.targets ?? '',
              req.quality ?? 'high'
            );
          }
        } catch {
          /* proyecto no montado / offline: se queda en la cola */
        }
      }
    })();
  }, [launch, hasSession]);

  return (
    <BackupContext.Provider
      value={{
        jobs,
        startBackup,
        retryBackup,
        dismissBackup,
        userLaunches,
        detail,
        openBackupDetail,
        closeBackupDetail,
      }}
    >
      {children}
    </BackupContext.Provider>
  );
}

export function useBackup(): BackupContextValue {
  const ctx = useContext(BackupContext);
  if (!ctx) throw new Error('useBackup debe usarse dentro de <BackupProvider>');
  return ctx;
}
