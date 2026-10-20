import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Input, Button } from '@lumik/ui';
import { getBackupServerUrl, setBackupServerUrl } from '../lib/api';
import { useBackupSession } from '../lib/BackupSessionContext';
import { BackupAccountForm } from '../components/BackupAccountForm';

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
  margin: 0,
};

/** Ajustes → Respaldo: la URL del servidor y la cuenta (login / registro, o el
 *  email con "Cerrar sesión"). Es el único sitio donde se gestiona la cuenta. */
export function BackupAccountSection() {
  const { t } = useTranslation();
  const { session, loading: sessionLoading, logout } = useBackupSession();
  const [serverUrl, setServerUrl] = useState('');
  const [savedUrl, setSavedUrl] = useState('');
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loggingOut, setLoggingOut] = useState(false);

  useEffect(() => {
    getBackupServerUrl()
      .then((url) => {
        setServerUrl(url);
        setSavedUrl(url);
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  const persistUrl = async () => {
    const url = serverUrl.trim();
    await setBackupServerUrl(url);
    setServerUrl(url);
    setSavedUrl(url);
  };

  const save = async () => {
    setError(null);
    setSaved(false);
    setBusy(true);
    try {
      await persistUrl();
      setSaved(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleLogout = async () => {
    setLoggingOut(true);
    try {
      await logout();
    } finally {
      setLoggingOut(false);
    }
  };

  return (
    <section style={{ display: 'flex', flexDirection: 'column', gap: '20px', maxWidth: '560px' }}>
      <div style={sectionHeaderStyles}>
        <h2 style={sectionTitleStyles}>{t('settings.backup.title')}</h2>
        <p style={sectionDescriptionStyles}>{t('settings.backup.description')}</p>
      </div>

      {loading || sessionLoading ? (
        <p style={mutedStyles}>{t('settings.loading')}</p>
      ) : (
        <div style={formStyles}>
          <Input
            label={t('settings.backup.serverUrl')}
            value={serverUrl}
            onChange={(e) => {
              setServerUrl(e.target.value);
              setSaved(false);
            }}
            fullWidth
          />
          {error && <span style={errorStyles}>{error}</span>}
          <div style={rowStyles}>
            <Button
              variant="secondary"
              size="sm"
              onClick={save}
              disabled={busy || serverUrl.trim() === savedUrl}
            >
              {t('settings.backup.saveUrl')}
            </Button>
            {saved && <span style={mutedStyles}>{t('settings.backup.urlSaved')}</span>}
          </div>
          {session ? (
            <div style={rowStyles}>
              <p style={{ ...mutedStyles, flex: 1 }}>
                {t('settings.backup.sessionStatus', { email: session.email, server: session.server_url })}
              </p>
              <Button variant="secondary" size="sm" onClick={handleLogout} disabled={loggingOut}>
                {t('settings.backup.logout')}
              </Button>
            </div>
          ) : (
            // Una URL editada y sin guardar se guarda antes de autenticar.
            <BackupAccountForm
              beforeSubmit={serverUrl.trim() !== savedUrl ? persistUrl : undefined}
            />
          )}
        </div>
      )}
    </section>
  );
}
