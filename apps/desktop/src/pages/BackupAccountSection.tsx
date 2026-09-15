import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Input, Button } from '@lumik/ui';
import {
  getBackupSession,
  getBackupServerUrl,
  setBackupServerUrl,
  backupLogin,
  backupRegister,
  backupLogout,
} from '../lib/api';
import type { BackupSession } from '../lib/types';

const sectionHeaderStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '4px',
};
const sectionTitleStyles: React.CSSProperties = {
  fontSize: '18px',
  fontWeight: 600,
  color: 'var(--lumik-on-surface, #e5e2e1)',
  margin: 0,
};
const sectionDescriptionStyles: React.CSSProperties = {
  fontSize: '14px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
  margin: 0,
};
const formStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '14px',
  maxWidth: '420px',
};
const rowStyles: React.CSSProperties = {
  display: 'flex',
  gap: '12px',
  alignItems: 'center',
};
const errorStyles: React.CSSProperties = {
  fontSize: '13px',
  color: 'var(--lumik-error, #ffb4ab)',
};
const mutedStyles: React.CSSProperties = {
  fontSize: '13px',
  color: 'var(--lumik-on-surface-variant, #c2c6d7)',
};

type Mode = 'login' | 'register';

export function BackupAccountSection() {
  const { t } = useTranslation();
  const [session, setSession] = useState<BackupSession | null>(null);
  const [serverUrl, setServerUrl] = useState('');
  const [mode, setMode] = useState<Mode>('login');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [name, setName] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [url, current] = await Promise.all([
        getBackupServerUrl(),
        getBackupSession().catch(() => null),
      ]);
      setServerUrl(url);
      setSession(current);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const submit = async () => {
    setError(null);
    if (!email || !password) {
      setError(t('settings.backup.errors.required'));
      return;
    }
    setBusy(true);
    try {
      // Guarda la URL del server antes de autenticar.
      await setBackupServerUrl(serverUrl.trim());
      const s =
        mode === 'login'
          ? await backupLogin(email, password)
          : await backupRegister(email, password, name || undefined);
      setSession(s);
      setPassword('');
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const logout = async () => {
    setBusy(true);
    try {
      await backupLogout();
      setSession(null);
    } finally {
      setBusy(false);
    }
  };

  return (
    <section style={{ display: 'flex', flexDirection: 'column', gap: '20px', maxWidth: '560px' }}>
      <div style={sectionHeaderStyles}>
        <h2 style={sectionTitleStyles}>{t('settings.backup.title')}</h2>
        <p style={sectionDescriptionStyles}>{t('settings.backup.description')}</p>
      </div>

      {loading ? (
        <p style={mutedStyles}>{t('settings.loading')}</p>
      ) : session ? (
        <div style={formStyles}>
          <p style={sectionDescriptionStyles}>
            {t('settings.backup.signedInAs', { email: session.email })}
          </p>
          <p style={mutedStyles}>
            {t('settings.backup.server')}: {session.server_url}
          </p>
          <div style={rowStyles}>
            <Button variant="secondary" size="sm" onClick={logout} disabled={busy}>
              {t('settings.backup.logout')}
            </Button>
          </div>
        </div>
      ) : (
        <div style={formStyles}>
          <Input
            label={t('settings.backup.serverUrl')}
            value={serverUrl}
            onChange={(e) => setServerUrl(e.target.value)}
            fullWidth
          />
          {mode === 'register' && (
            <Input
              label={t('settings.backup.name')}
              value={name}
              onChange={(e) => setName(e.target.value)}
              fullWidth
            />
          )}
          <Input
            label={t('settings.backup.email')}
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            fullWidth
          />
          <Input
            label={t('settings.backup.password')}
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            fullWidth
          />
          {error && <span style={errorStyles}>{error}</span>}
          <div style={rowStyles}>
            <Button variant="primary" onClick={submit} disabled={busy}>
              {mode === 'login'
                ? t('settings.backup.login')
                : t('settings.backup.register')}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setMode(mode === 'login' ? 'register' : 'login');
                setError(null);
              }}
              disabled={busy}
            >
              {mode === 'login'
                ? t('settings.backup.switchToRegister')
                : t('settings.backup.switchToLogin')}
            </Button>
          </div>
        </div>
      )}
    </section>
  );
}
