import {
  createContext,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from 'react';
import { authApi, tokens, type Photographer } from '../api/client';

interface AuthContextValue {
  photographer: Photographer | null;
  loading: boolean;
  login: (email: string, password: string) => Promise<void>;
  register: (email: string, password: string, name?: string) => Promise<void>;
  logout: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [photographer, setPhotographer] = useState<Photographer | null>(null);
  const [loading, setLoading] = useState(true);

  // Hidrata la sesión si hay un token guardado.
  useEffect(() => {
    if (!tokens.getAccess()) {
      setLoading(false);
      return;
    }
    authApi
      .me()
      .then(setPhotographer)
      .catch(() => tokens.clear())
      .finally(() => setLoading(false));
  }, []);

  const login = async (email: string, password: string) => {
    const res = await authApi.login(email, password);
    tokens.set(res.access, res.refresh);
    setPhotographer(res.photographer);
  };

  const register = async (email: string, password: string, name?: string) => {
    const res = await authApi.register(email, password, name);
    tokens.set(res.access, res.refresh);
    setPhotographer(res.photographer);
  };

  const logout = async () => {
    const refresh = tokens.getRefresh();
    try {
      if (refresh) await authApi.logout(refresh);
    } finally {
      tokens.clear();
      setPhotographer(null);
    }
  };

  return (
    <AuthContext.Provider value={{ photographer, loading, login, register, logout }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth debe usarse dentro de <AuthProvider>');
  return ctx;
}
