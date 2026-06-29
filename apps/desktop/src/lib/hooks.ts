import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import * as api from './api';
import type {
  AppSettings,
  DetectedDevice,
  KeybindingMap,
  Photo,
  ProjectDashboard,
  Photographer,
  ImportRequest,
  ImportLogEntry,
  ImportProgress,
  ImportResult,
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

export function usePhotoPreview(photoId: string, projectId: string) {
  const [state, setState] = useState<AsyncState<api.PhotoPreviewResult>>({
    data: null,
    loading: true,
    error: null,
  });

  useEffect(() => {
    let cancelled = false;
    setState({ data: null, loading: true, error: null });
    api
      .getPhotoPreview(photoId, projectId)
      .then((result) => {
        if (!cancelled) setState({ data: result, loading: false, error: null });
      })
      .catch((err) => {
        if (!cancelled) setState({ data: null, loading: false, error: String(err) });
      });
    return () => {
      cancelled = true;
    };
  }, [photoId, projectId]);

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
  'projects.new_project':      'n',
  'projects.focus_search':     's',
  'project.show_culled':       'Ctrl+c',
  'project.back':              'Escape',
  'project.import':            'i',
};

/**
 * Check if a KeyboardEvent matches a stored keybinding value.
 * Supports simple keys ("Escape", "+") and Ctrl+ combos ("Ctrl+c").
 * Plain keys require no modifier keys to be pressed.
 */
export function matchesKey(e: KeyboardEvent, stored: string): boolean {
  if (stored.startsWith('Ctrl+')) {
    return e.ctrlKey && !e.altKey && !e.metaKey && e.key === stored.slice(5);
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

interface UseImportReturn {
  progress: ImportProgress | null;
  importLog: string[];
  result: ImportResult | null;
  isImporting: boolean;
  error: string | null;
  startImport: (request: ImportRequest) => Promise<void>;
  reset: () => void;
}

export function useImport(): UseImportReturn {
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [importLog, setImportLog] = useState<string[]>([]);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const unlistenProgressRef = useRef<UnlistenFn | null>(null);
  const unlistenLogRef = useRef<UnlistenFn | null>(null);
  const logBufferRef = useRef<string[]>([]);
  const flushTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const sessionIdRef = useRef<string | null>(null);

  // Cleanup listeners on unmount
  useEffect(() => {
    return () => {
      unlistenProgressRef.current?.();
      unlistenLogRef.current?.();
      if (flushTimerRef.current !== null) clearInterval(flushTimerRef.current);
    };
  }, []);

  const startImport = useCallback(async (request: ImportRequest) => {
    setIsImporting(true);
    setError(null);
    setResult(null);
    setProgress(null);
    setImportLog([]);
    logBufferRef.current = [];
    sessionIdRef.current = request.session_id;

    try {
      unlistenProgressRef.current = await listen<ImportProgress>('import-progress', (event) => {
        if (event.payload.session_id === sessionIdRef.current) {
          setProgress(event.payload);
        }
      });

      unlistenLogRef.current = await listen<ImportLogEntry>('import-log', (event) => {
        if (event.payload.session_id === sessionIdRef.current) {
          logBufferRef.current.push(event.payload.message);
        }
      });

      // Flush log buffer to state every 150ms to avoid per-event re-renders
      flushTimerRef.current = setInterval(() => {
        if (logBufferRef.current.length > 0) {
          const pending = logBufferRef.current;
          logBufferRef.current = [];
          setImportLog((prev) => [...prev, ...pending]);
        }
      }, 150);
    } catch (err) {
      console.error('Failed to setup import listeners:', err);
    }

    try {
      const importResult = await api.startImport(request);
      setResult(importResult);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsImporting(false);
      unlistenProgressRef.current?.();
      unlistenProgressRef.current = null;
      unlistenLogRef.current?.();
      unlistenLogRef.current = null;
      if (flushTimerRef.current !== null) {
        clearInterval(flushTimerRef.current);
        flushTimerRef.current = null;
      }
      // Final flush to capture any events that arrived before cleanup
      if (logBufferRef.current.length > 0) {
        const pending = logBufferRef.current;
        logBufferRef.current = [];
        setImportLog((prev) => [...prev, ...pending]);
      }
    }
  }, []);

  const reset = useCallback(() => {
    setProgress(null);
    setImportLog([]);
    setResult(null);
    setError(null);
    setIsImporting(false);
    logBufferRef.current = [];
    sessionIdRef.current = null;
  }, []);

  return { progress, importLog, result, isImporting, error, startImport, reset };
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
