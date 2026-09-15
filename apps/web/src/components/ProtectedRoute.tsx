import { type ReactNode } from 'react';
import { Navigate } from 'react-router-dom';
import { Loader } from '@lumik/ui';
import { useAuth } from '../auth/AuthContext';

/** Solo renderiza `children` si hay sesión; si no, redirige a /login. */
export function ProtectedRoute({ children }: { children: ReactNode }) {
  const { photographer, loading } = useAuth();

  if (loading) {
    return (
      <div className="auth-shell">
        <Loader />
      </div>
    );
  }
  if (!photographer) return <Navigate to="/login" replace />;
  return <>{children}</>;
}
