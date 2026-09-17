import { Navigate, Outlet, Link, useLocation } from 'react-router-dom';
import { Loader, Logo, Button } from '@lumik/ui';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../auth/AuthContext';

/** Shell de las páginas autenticadas: exige sesión y muestra la navegación. */
export function ProtectedLayout() {
  const { photographer, loading, logout } = useAuth();
  const { t } = useTranslation();
  const loc = useLocation();

  if (loading) {
    return (
      <div className="auth-shell">
        <Loader />
      </div>
    );
  }
  if (!photographer) return <Navigate to="/login" replace />;

  const navLink = (to: string, label: string) => (
    <Link to={to} className={loc.pathname === to ? 'nav-link nav-link--active' : 'nav-link'}>
      {label}
    </Link>
  );

  return (
    <div className="app-shell">
      <header className="app-header">
        <div className="app-header__left">
          <Logo />
          <nav className="app-nav">
            {navLink('/', t('nav.projects'))}
            {navLink('/config', t('nav.config'))}
          </nav>
        </div>
        <div className="app-header__right">
          <span className="subtitle">{photographer.email}</span>
          <Button variant="secondary" size="sm" onClick={logout}>
            {t('nav.logout')}
          </Button>
        </div>
      </header>
      <main className="app-main">
        <Outlet />
      </main>
    </div>
  );
}
