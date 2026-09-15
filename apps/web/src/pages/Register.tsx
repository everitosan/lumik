import { useState, type FormEvent } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Button, Input, Logo } from '@lumik/ui';
import { useAuth } from '../auth/AuthContext';
import { ApiError } from '../api/client';

export default function Register() {
  const { t } = useTranslation();
  const { register } = useAuth();
  const navigate = useNavigate();
  const [name, setName] = useState('');
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
    if (password.length < 8) {
      setError(t('auth.errors.passwordShort'));
      return;
    }
    setBusy(true);
    try {
      await register(email, password, name || undefined);
      navigate('/', { replace: true });
    } catch (err) {
      if (err instanceof ApiError && err.status === 409) {
        setError(t('auth.errors.emailTaken'));
      } else if (err instanceof ApiError && err.status === 400) {
        setError(t('auth.errors.passwordShort'));
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
          <h1>{t('auth.register.title')}</h1>
          <p className="subtitle">{t('auth.register.subtitle')}</p>
        </div>
        <div className="auth-form">
          <Input
            label={t('auth.fields.name')}
            value={name}
            onChange={(e) => setName(e.target.value)}
            fullWidth
          />
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
            autoComplete="new-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            fullWidth
          />
          {error && <span className="auth-error">{error}</span>}
          <Button type="submit" variant="primary" fullWidth disabled={busy}>
            {t('auth.register.submit')}
          </Button>
        </div>
        <p className="auth-switch">
          {t('auth.register.hasAccount')}{' '}
          <Link to="/login">{t('auth.register.goLogin')}</Link>
        </p>
      </form>
    </div>
  );
}
