import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import * as api from './api';
import { DEFAULT_DASHBOARD_SORT } from './types';
import type {
  AppSettings,
  DashboardSort,
  DetectedDevice,
  KeybindingMap,
  Photo,
  ProjectDashboard,
  Photographer,
  FailedFile,
  ImportRequest,
  ImportLogEntry,
  ImportProgress,
  ImportResult,
  RetryRegisterRequest,
} from './types';

interface AsyncState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

// ============================================================================
// PLATFORM
// ============================================================================

export function usePlatform() {
  const [platform, setPlatform] = useState<api.Platform | null>(null);
  useEffect(() => { api.getPlatform().then(setPlatform); }, []);
  return platform;
}

// ============================================================================
// DEVICE HOOKS (runtime scan with polling)
// ============================================================================

// Desktop relies on the native OS hotplug watcher (emits "devices-changed"),
// so it only needs a slow safety-net poll. Android has no native listener and
// keeps a responsive poll.
const DESKTOP_POLL_INTERVAL = 60000; // 60s fallback in case an event is missed
const ANDROID_POLL_INTERVAL = 10000; // 10s — Android has no OS listener

export function useConnectedDevices(pollIntervalOverride?: number) {
  const platform = usePlatform();
  const pollInterval =
    pollIntervalOverride ??
    (platform === 'android' ? ANDROID_POLL_INTERVAL : DESKTOP_POLL_INTERVAL);

  const [state, setState] = useState<AsyncState<DetectedDevice[]>>({
    data: null,
    loading: true,
    error: null,
  });
  const isFirstLoad = useRef(true);

  const refetch = useCallback(async (silent = false) => {
    if (!silent) {
      setState((s) => ({ ...s, loading: true, error: null }));
    }
    try {
      const data = await api.scanConnectedDevices();
      setState({ data, loading: false, error: null });
    } catch (err) {
      if (!silent) {
        setState({ data: null, loading: false, error: String(err) });
      }
    }
  }, []);

  // Initial fetch
  useEffect(() => {
    refetch(false);
    isFirstLoad.current = false;
  }, [refetch]);

  // Polling
  useEffect(() => {
    if (pollInterval <= 0) return;

    const interval = setInterval(() => {
      refetch(true); // Silent refresh
    }, pollInterval);

    return () => clearInterval(interval);
  }, [refetch, pollInterval]);

  // Refresh immediately when the backend signals a device change (e.g. an eject
  // from elsewhere in the UI), so every useConnectedDevices instance stays in
  // sync without waiting for its own poll tick.
  useEffect(() => {
    let unlisten: UnlistenFn | undefined;
    listen('devices-changed', () => refetch(true)).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [refetch]);

  // Safely eject a device. The backend emits "devices-changed" on success, which
  // refreshes this (and every other) device list, so no manual refetch is needed.
  const eject = useCallback(
    async (deviceUuid: string) => {
      await api.ejectDevice(deviceUuid);
    },
    [],
  );

  return { ...state, refetch: () => refetch(false), eject };
}

// ============================================================================
// PROJECT HOOKS
// ============================================================================

export function useProjectsDashboard() {
  const [state, setState] = useState<AsyncState<ProjectDashboard[]>>({
    data: null,
    loading: true,
    error: null,
  });

  const refetch = useCallback(async () => {
    setState((s) => ({ ...s, loading: true, error: null }));
    try {
      const data = await api.getProjectsDashboard();
      setState({ data, loading: false, error: null });
    } catch (err) {
      setState({ data: null, loading: false, error: String(err) });
    }
  }, []);

  useEffect(() => {
    refetch();
  }, [refetch]);

  return { ...state, refetch };
}

// ============================================================================
// PHOTO HOOKS
// ============================================================================

export function useProjectPhotos(projectId: string) {
  const [state, setState] = useState<AsyncState<Photo[]>>({
    data: null,
    loading: true,
    error: null,
  });

  const refetch = useCallback(async () => {
    setState((s) => ({ ...s, loading: true, error: null }));
    try {
      const data = await api.getProjectPhotos(projectId);
      setState({ data, loading: false, error: null });
    } catch (err) {
      setState({ data: null, loading: false, error: String(err) });
    }
  }, [projectId]);

  useEffect(() => {
    refetch();
  }, [refetch]);

  return { ...state, refetch };
}

export function useProjectTrashedPhotos(projectId: string) {
  const [state, setState] = useState<AsyncState<Photo[]>>({
    data: null,
    loading: true,
    error: null,
  });

  const refetch = useCallback(async () => {
    setState((s) => ({ ...s, loading: true, error: null }));
    try {
      const data = await api.getProjectTrashedPhotos(projectId);
      setState({ data, loading: false, error: null });
    } catch (err) {
      setState({ data: null, loading: false, error: String(err) });
    }
  }, [projectId]);

  useEffect(() => {
    refetch();
  }, [refetch]);

  return { ...state, refetch };
}

export function useProjectThumbnails(projectId: string) {
  const [thumbnailIds, setThumbnailIds] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);

  const refetch = useCallback(async () => {
    setLoading(true);
    try {
      const ids = await api.getProjectThumbnails(projectId);
      setThumbnailIds(new Set(ids));
    } catch {
      setThumbnailIds(new Set());
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    refetch();
  }, [refetch]);

  return { thumbnailIds, loading, refetch };
}

export function useThumbnail(projectId: string, photoId: string, enabled: boolean) {
  const [url, setUrl] = useState<string | null>(null);

  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    api.getThumbnail(projectId, photoId).then((data) => {
      if (!cancelled) setUrl(data);
    });
    return () => { cancelled = true; };
  }, [projectId, photoId, enabled]);

  return url;
}

export function useCoverThumbnails(projects: ProjectDashboard[] | null): Record<string, string> {
  const [thumbnails, setThumbnails] = useState<Record<string, string>>({});

  useEffect(() => {
    if (!projects) return;
    const withCover = projects.filter((p) => p.cover_photo_path);
    if (withCover.length === 0) { setThumbnails({}); return; }

    let cancelled = false;
    Promise.all(
      withCover.map(async (p) => {
        const url = await api.getProjectCoverThumbnail(p.id).catch(() => null);
        return [p.id, url] as const;
      })
    ).then((results) => {
      if (cancelled) return;
      const map: Record<string, string> = {};
      for (const [projectId, url] of results) {
        if (url) map[projectId] = url;
      }
      setThumbnails(map);
    });

    return () => { cancelled = true; };
  }, [projects]);

  return thumbnails;
}

// ============================================================================
// PHOTOGRAPHER HOOKS
// ============================================================================

export function useActivePhotographer() {
  const [state, setState] = useState<AsyncState<Photographer>>({
    data: null,
    loading: true,
    error: null,
  });

  const refetch = useCallback(async () => {
    setState((s) => ({ ...s, loading: true, error: null }));
    try {
      const data = await api.getActivePhotographer();
      setState({ data, loading: false, error: null });
    } catch (err) {
      setState({ data: null, loading: false, error: String(err) });
    }
  }, []);

  useEffect(() => {
    refetch();
  }, [refetch]);

  return { ...state, refetch };
}

// ============================================================================
// PHOTO PREVIEW HOOK
// ============================================================================

/** Vista previa de una foto; de un par RAW+JPG, su JPG salvo que se pida el RAW. */
export function usePhotoPreview(photoId: string, projectId: string, raw = false) {
  const [state, setState] = useState<AsyncState<api.PhotoPreviewResult>>({
    data: null,
    loading: true,
    error: null,
  });

  useEffect(() => {
    let cancelled = false;
    setState({ data: null, loading: true, error: null });
    api
      .getPhotoPreview(photoId, projectId, raw)
      .then((result) => {
        if (!cancelled) setState({ data: result, loading: false, error: null });
      })
      .catch((err) => {
        if (!cancelled) setState({ data: null, loading: false, error: String(err) });
      });
    return () => {
      cancelled = true;
    };
  }, [photoId, projectId, raw]);

  return state;
}

// ============================================================================
// SETTINGS HOOKS
// ============================================================================

export function useAppSettings() {
  const [state, setState] = useState<AsyncState<AppSettings>>({
    data: null,
    loading: true,
    error: null,
  });

  const refetch = useCallback(async () => {
    setState((s) => ({ ...s, loading: true, error: null }));
    try {
      const data = await api.getAppSettings();
      setState({ data, loading: false, error: null });
    } catch (err) {
      setState({ data: null, loading: false, error: String(err) });
    }
  }, []);

  useEffect(() => {
    refetch();
  }, [refetch]);

  return { ...state, refetch };
}

/**
 * Orden del dashboard. Mientras carga usa el orden por defecto (el histórico),
 * así no hay salto visual. Los cambios se aplican al momento y se guardan; si
 * guardar falla, vuelve al valor anterior.
 */
export function useDashboardSort() {
  const [sort, setSortState] = useState<DashboardSort>(DEFAULT_DASHBOARD_SORT);
  const sortRef = useRef(sort);
  sortRef.current = sort;
  // Si el usuario cambia el orden antes de que llegue el guardado, gana su cambio.
  const touchedRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    api.getDashboardSort()
      .then((stored) => {
        if (!cancelled && !touchedRef.current) setSortState(stored);
      })
      .catch((err) => console.error('getDashboardSort failed:', err));
    return () => {
      cancelled = true;
    };
  }, []);

  const setSort = useCallback((next: DashboardSort) => {
    const previous = sortRef.current;
    touchedRef.current = true;
    setSortState(next);
    api.updateDashboardSort(next).catch((err) => {
      console.error('updateDashboardSort failed:', err);
      setSortState(previous);
    });
  }, []);

  return { sort, setSort };
}

/**
 * Si el dashboard muestra los proyectos archivados (por defecto, no). Mismo
 * comportamiento que `useDashboardSort`: optimista, vuelve atrás si guardar
 * falla y el cambio del usuario gana a una carga tardía.
 */
export function useDashboardShowArchived() {
  const [showArchived, setState] = useState(false);
  const valueRef = useRef(showArchived);
  valueRef.current = showArchived;
  const touchedRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    api.getDashboardShowArchived()
      .then((stored) => {
        if (!cancelled && !touchedRef.current) setState(stored);
      })
      .catch((err) => console.error('getDashboardShowArchived failed:', err));
    return () => {
      cancelled = true;
    };
  }, []);

  const setShowArchived = useCallback((next: boolean) => {
    const previous = valueRef.current;
    touchedRef.current = true;
    setState(next);
    api.setDashboardShowArchived(next).catch((err) => {
      console.error('setDashboardShowArchived failed:', err);
      setState(previous);
    });
  }, []);

  return { showArchived, setShowArchived };
}

// ============================================================================
// KEYBINDING HOOK
// ============================================================================

const DEFAULT_KEYBINDINGS: KeybindingMap = {
  'photo_detail.close':        'Escape',
  'photo_detail.prev':         'ArrowLeft',
  'photo_detail.next':         'ArrowRight',
  'photo_detail.zoom_in':      '+',
  'photo_detail.zoom_out':     '-',
  'photo_detail.fit':          'f',
  'photo_detail.rotate_left':  '[',
  'photo_detail.rotate_right': ']',
  'photo_detail.cull':         ' ',
  'photo_detail.stars_0':      '0',
  'photo_detail.stars_1':      '1',
  'photo_detail.stars_2':      '2',
  'photo_detail.stars_3':      '3',
  'photo_detail.stars_4':      '4',
  'photo_detail.stars_5':      '5',
  'photo_detail.add_tag':      't',
  'photo_detail.toggle_raw':   's',
  'projects.new_project':      'n',
  'projects.focus_search':     's',
  'project.show_culled':       'Ctrl+c',
  'project.back':              'Escape',
  'project.import':            'i',
};

/**
 * True on macOS. Synchronous (unlike `usePlatform`) so key handlers can use it;
 * the WebView reports `MacIntel` on Apple Silicon too.
 */
export const IS_MAC = typeof navigator !== 'undefined'
  && /Mac/.test(navigator.platform || navigator.userAgent);

/**
 * Whether the platform modifier (Cmd on macOS, Ctrl elsewhere) is pressed on
 * its own: Alt and the other platform's modifier must be up. On macOS
 * Ctrl+click is a secondary click, so Ctrl never stands in for Cmd.
 */
export function hasPlatformModifier(e: KeyboardEvent): boolean {
  const mod = IS_MAC ? e.metaKey : e.ctrlKey;
  const foreign = IS_MAC ? e.ctrlKey : e.metaKey;
  return mod && !foreign && !e.altKey;
}

/**
 * Check if a KeyboardEvent matches a stored keybinding value.
 * Supports simple keys ("Escape", "+") and modifier combos stored as "Ctrl+c",
 * which fire with Cmd on macOS and Ctrl elsewhere.
 * Plain keys require no modifier keys to be pressed.
 */
export function matchesKey(e: KeyboardEvent, stored: string): boolean {
  if (stored.startsWith('Ctrl+')) {
    return hasPlatformModifier(e) && e.key === stored.slice(5);
  }
  return e.key === stored && !e.ctrlKey && !e.altKey && !e.metaKey;
}

export function useKeybindings(): KeybindingMap {
  const [map, setMap] = useState<KeybindingMap>(DEFAULT_KEYBINDINGS);

  useEffect(() => {
    api.getKeybindings().then((rows) => {
      setMap(Object.fromEntries(rows.map((r) => [r.action, r.key])));
    }).catch(() => {
      // Keep defaults on error
    });
  }, []);

  return map;
}

/**
 * Returns only the keybindings whose action starts with `context + "."`.
 * Example: useContextKeybindings('photo_detail') returns all photo_detail.* bindings.
 * Full action names are preserved as keys so existing handler code works unchanged.
 */
export function useContextKeybindings(context: string): KeybindingMap {
  const all = useKeybindings();
  return useMemo(
    () => Object.fromEntries(
      Object.entries(all).filter(([action]) => action.startsWith(`${context}.`))
    ),
    [all, context],
  );
}

// ============================================================================
// IMPORT HOOKS
// ============================================================================

/**
 * `idle`: sin importar. `importing`: en curso. `done`: terminó, con `result`
 * (que puede traer `fatal` si terminó antes de tiempo) o con `error` si el
 * import ni siquiera pudo empezar.
 */
export type ImportStatus = 'idle' | 'importing' | 'done';

/**
 * Una tanda de un reintento: `register` vuelve a registrar fotos que ya están en
 * `_media/`; `import` vuelve a importar desde el origen. `files` son los fallidos
 * del resultado anterior que cubre la tanda.
 */
export type ImportRetryStep =
  | { kind: 'register'; request: RetryRegisterRequest; files: FailedFile[] }
  | { kind: 'import'; request: ImportRequest; files: FailedFile[] };

interface UseImportReturn {
  status: ImportStatus;
  progress: ImportProgress | null;
  importLog: ImportLogEntry[];
  result: ImportResult | null;
  isImporting: boolean;
  error: string | null;
  startImport: (request: ImportRequest) => Promise<void>;
  /** Reintenta en serie; conserva el registro y suma al resultado anterior. */
  retry: (steps: ImportRetryStep[]) => Promise<void>;
  reset: () => void;
}

const failedKey = (f: FailedFile) => `${f.code}|${f.path}`;

/**
 * Total de la sesión tras un reintento: suma lo importado, quita de los fallidos
 * los que se reintentaron (`retried`) y agrega los que volvieron a fallar.
 * Los ignorados y el total de archivos son los del primer intento.
 */
export function mergeRetryResult(prev: ImportResult, retried: FailedFile[], next: ImportResult): ImportResult {
  const retriedKeys = new Set(retried.map(failedKey));
  const failed_files = [...prev.failed_files.filter((f) => !retriedKeys.has(failedKey(f))), ...next.failed_files];
  return {
    ...prev,
    successful: prev.successful + next.successful,
    videos_copied: prev.videos_copied + next.videos_copied,
    joined: (prev.joined ?? 0) + (next.joined ?? 0),
    failed: failed_files.length,
    failed_files,
    fatal: next.fatal ?? prev.fatal,
  };
}

export function useImport(): UseImportReturn {
  const [status, setStatus] = useState<ImportStatus>('idle');
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [importLog, setImportLog] = useState<ImportLogEntry[]>([]);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const unlistenProgressRef = useRef<UnlistenFn | null>(null);
  const unlistenLogRef = useRef<UnlistenFn | null>(null);
  const logBufferRef = useRef<ImportLogEntry[]>([]);
  const flushTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const sessionIdRef = useRef<string | null>(null);
  const resultRef = useRef<ImportResult | null>(null);

  // Cleanup listeners on unmount
  useEffect(() => {
    return () => {
      unlistenProgressRef.current?.();
      unlistenLogRef.current?.();
      if (flushTimerRef.current !== null) clearInterval(flushTimerRef.current);
    };
  }, []);

  const flushLog = useCallback(() => {
    if (logBufferRef.current.length > 0) {
      const pending = logBufferRef.current;
      logBufferRef.current = [];
      setImportLog((prev) => [...prev, ...pending]);
    }
  }, []);

  // Escucha los eventos del import; `sessionIdRef` decide de qué sesión.
  const startListening = useCallback(async () => {
    try {
      unlistenProgressRef.current = await listen<ImportProgress>('import-progress', (event) => {
        if (event.payload.session_id === sessionIdRef.current) {
          setProgress(event.payload);
        }
      });

      unlistenLogRef.current = await listen<ImportLogEntry>('import-log', (event) => {
        if (event.payload.session_id === sessionIdRef.current) {
          logBufferRef.current.push(event.payload);
        }
      });

      // Flush log buffer to state every 150ms to avoid per-event re-renders
      flushTimerRef.current = setInterval(flushLog, 150);
    } catch (err) {
      console.error('Failed to setup import listeners:', err);
    }
  }, [flushLog]);

  const stopListening = useCallback(() => {
    unlistenProgressRef.current?.();
    unlistenProgressRef.current = null;
    unlistenLogRef.current?.();
    unlistenLogRef.current = null;
    if (flushTimerRef.current !== null) {
      clearInterval(flushTimerRef.current);
      flushTimerRef.current = null;
    }
    // Final flush to capture any events that arrived before cleanup
    flushLog();
  }, [flushLog]);

  const startImport = useCallback(async (request: ImportRequest) => {
    setStatus('importing');
    setError(null);
    setResult(null);
    resultRef.current = null;
    setProgress(null);
    setImportLog([]);
    logBufferRef.current = [];
    sessionIdRef.current = request.session_id;

    await startListening();
    try {
      const importResult = await api.startImport(request);
      resultRef.current = importResult;
      setResult(importResult);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setStatus('done');
      stopListening();
    }
  }, [startListening, stopListening]);

  const retry = useCallback(async (steps: ImportRetryStep[]) => {
    if (!resultRef.current || steps.length === 0) return;
    setStatus('importing');
    setError(null);
    setProgress(null);

    await startListening();
    try {
      for (const step of steps) {
        sessionIdRef.current = step.request.session_id;
        setProgress(null);
        // Un `Err` (precondición) corta el reintento y deja el resultado como estaba.
        const next = step.kind === 'register'
          ? await api.retryImportRegister(step.request)
          : await api.startImport(step.request);
        const merged = mergeRetryResult(resultRef.current!, step.files, next);
        resultRef.current = merged;
        setResult(merged);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setStatus('done');
      stopListening();
    }
  }, [startListening, stopListening]);

  const reset = useCallback(() => {
    setStatus('idle');
    setProgress(null);
    setImportLog([]);
    setResult(null);
    resultRef.current = null;
    setError(null);
    logBufferRef.current = [];
    sessionIdRef.current = null;
  }, []);

  return {
    status,
    progress,
    importLog,
    result,
    isImporting: status === 'importing',
    error,
    startImport,
    retry,
    reset,
  };
}

// ============================================================================
// LANGUAGE
// ============================================================================

export function useLanguage() {
  const { i18n } = useTranslation();

  const changeLanguage = useCallback((lang: 'es' | 'en') => {
    i18n.changeLanguage(lang);
    localStorage.setItem('lumik-language', lang);
  }, [i18n]);

  return {
    currentLanguage: (i18n.language || 'es') as 'es' | 'en',
    changeLanguage,
  };
}
