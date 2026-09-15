import { useState, type FormEvent } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Button, Input, Logo } from '@lumik/ui';
import { useAuth } from '../auth/AuthContext';
import { ApiError } from '../api/client';

export default function Login() {
  const { t } = useTranslation();
  const { login } = useAuth();
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    if (!email || !password) {
      setError(t('auth.errors.required'));
      return;
    }
    setBusy(true);
    try {
      await login(email, password);
      navigate('/', { replace: true });
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) {
        setError(t('auth.errors.invalidCredentials'));
      } else {
        setError(t('auth.errors.generic'));
      }
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="auth-shell">
      <form className="auth-card" onSubmit={onSubmit}>
        <Logo />
        <div>
          <h1>{t('auth.login.title')}</h1>
          <p className="subtitle">{t('auth.login.subtitle')}</p>
        </div>
        <div className="auth-form">
          <Input
            label={t('auth.fields.email')}
            type="email"
            autoComplete="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            fullWidth
          />
          <Input
            label={t('auth.fields.password')}
            type="password"
            autoComplete="current-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            fullWidth
          />
          {error && <span className="auth-error">{error}</span>}
          <Button type="submit" variant="primary" fullWidth disabled={busy}>
            {t('auth.login.submit')}
          </Button>
        </div>
        <p className="auth-switch">
          {t('auth.login.noAccount')}{' '}
          <Link to="/register">{t('auth.login.goRegister')}</Link>
        </p>
      </form>
    </div>
  );
}
