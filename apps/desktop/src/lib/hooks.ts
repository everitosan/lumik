import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
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
  ImportProgress,
  ImportResult,
} from './types';

interface AsyncState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

// ============================================================================
// DEVICE HOOKS (runtime scan with polling)
// ============================================================================

const DEFAULT_POLL_INTERVAL = 10000; // 10 seconds

export function useConnectedDevices(pollInterval = DEFAULT_POLL_INTERVAL) {
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

  return { ...state, refetch: () => refetch(false) };
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
  const [thumbnails, setThumbnails] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(true);

  const refetch = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.getProjectThumbnails(projectId);
      setThumbnails(data);
    } catch {
      setThumbnails({});
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    refetch();
  }, [refetch]);

  return { thumbnails, loading, refetch };
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
  'photo_detail.fit':          '0',
  'photo_detail.rotate_left':  '[',
  'photo_detail.rotate_right': ']',
  'photo_detail.cull':         ' ',
  'project.show_culled':       'Ctrl+c',
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
  result: ImportResult | null;
  isImporting: boolean;
  error: string | null;
  startImport: (request: ImportRequest) => Promise<void>;
  reset: () => void;
}

export function useImport(): UseImportReturn {
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [isImporting, setIsImporting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);
  const sessionIdRef = useRef<string | null>(null);

  // Cleanup listener on unmount
  useEffect(() => {
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
      }
    };
  }, []);

  const startImport = useCallback(async (request: ImportRequest) => {
    setIsImporting(true);
    setError(null);
    setResult(null);
    setProgress(null);
    sessionIdRef.current = request.session_id;

    // Setup progress listener before starting import
    try {
      unlistenRef.current = await listen<ImportProgress>('import-progress', (event) => {
        // Only process events for our session
        if (event.payload.session_id === sessionIdRef.current) {
          setProgress(event.payload);
        }
      });
    } catch (err) {
      console.error('Failed to setup progress listener:', err);
    }

    try {
      const importResult = await api.startImport(request);
      setResult(importResult);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setIsImporting(false);
      // Cleanup listener
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    }
  }, []);

  const reset = useCallback(() => {
    setProgress(null);
    setResult(null);
    setError(null);
    setIsImporting(false);
    sessionIdRef.current = null;
  }, []);

  return { progress, result, isImporting, error, startImport, reset };
}
