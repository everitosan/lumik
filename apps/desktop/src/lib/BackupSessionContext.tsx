import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from 'react';
import * as api from './api';
import type { BackupSession } from './types';

interface BackupSessionValue {
  /** Sesión de la cuenta de respaldo, o `null` si no hay. */
  session: BackupSession | null;
  /** Cargando la sesión guardada al arrancar. */
  loading: boolean;
  refresh: () => Promise<void>;
  login: (email: string, password: string) => Promise<void>;
  register: (email: string, password: string, name?: string) => Promise<void>;
  logout: () => Promise<void>;
}

const BackupSessionContext = createContext<BackupSessionValue | null>(null);

/**
 * Estado único de la sesión de respaldo: la barra lateral, el modal de cuenta y
 * Ajustes → Respaldo la leen y la cambian aquí, así que iniciar o cerrar sesión
 * en un sitio se refleja al momento en los demás.
 */
export function BackupSessionProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<BackupSession | null>(null);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      setSession(await api.getBackupSession());
    } catch {
      setSession(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const login = useCallback(async (email: string, password: string) => {
    setSession(await api.backupLogin(email, password));
  }, []);

  const register = useCallback(async (email: string, password: string, name?: string) => {
    setSession(await api.backupRegister(email, password, name));
  }, []);

  const logout = useCallback(async () => {
    try {
      await api.backupLogout();
    } finally {
      // El cierre de sesión local no depende de que el servidor responda.
      setSession(null);
    }
  }, []);

  return (
    <BackupSessionContext.Provider value={{ session, loading, refresh, login, register, logout }}>
      {children}
    </BackupSessionContext.Provider>
  );
}

export function useBackupSession(): BackupSessionValue {
  const ctx = useContext(BackupSessionContext);
  if (!ctx) throw new Error('useBackupSession debe usarse dentro de <BackupSessionProvider>');
  return ctx;
}
