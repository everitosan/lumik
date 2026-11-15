// Lo que hay en la papelera de los discos conectados (spec `trash-bin`): lo usan
// la entrada "Papelera" del menú (solo existe si hay algo) y su página. Se
// recalcula al cambiar los discos y cada vez que `api.ts` avisa que una acción
// mandó a la papelera, restauró o eliminó algo. No se guarda en ningún lado.
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';
import * as api from './api';
import { useConnectedDevices } from './hooks';
import type { DiskTrash } from './types';

interface TrashState {
  /** Un grupo por disco conectado con algo en la papelera; `null` mientras carga. */
  disks: DiskTrash[] | null;
  /** Proyectos + fotos sueltas en la papelera de los discos conectados. */
  count: number;
  error: string | null;
  refresh: () => Promise<void>;
}

const TrashContext = createContext<TrashState | null>(null);

export function trashCount(disks: DiskTrash[] | null): number {
  return (disks ?? []).reduce((n, d) => n + d.projects.length + d.photos.length, 0);
}

export function TrashProvider({ children }: { children: ReactNode }) {
  const [disks, setDisks] = useState<DiskTrash[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { data: devices } = useConnectedDevices();

  const refresh = useCallback(async () => {
    try {
      setDisks(await api.listTrash());
      setError(null);
    } catch (err) {
      setError(String(err));
    }
  }, []);

  // Al arrancar y cada vez que cambian los discos conectados.
  const loaded = devices !== null;
  const deviceKey = (devices ?? []).map((d) => d.uuid).sort().join(',');
  useEffect(() => {
    if (loaded) void refresh();
  }, [deviceKey, loaded, refresh]);

  useEffect(() => {
    const onChange = () => void refresh();
    window.addEventListener(api.TRASH_CHANGED_EVENT, onChange);
    return () => window.removeEventListener(api.TRASH_CHANGED_EVENT, onChange);
  }, [refresh]);

  const value = useMemo(() => ({ disks, count: trashCount(disks), error, refresh }), [disks, error, refresh]);
  return <TrashContext.Provider value={value}>{children}</TrashContext.Provider>;
}

export function useTrash(): TrashState {
  const ctx = useContext(TrashContext);
  if (!ctx) throw new Error('useTrash fuera de TrashProvider');
  return ctx;
}

/** Solo el conteo, para la entrada del menú. */
export function useTrashCount(): number {
  return useTrash().count;
}
