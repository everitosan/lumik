import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Input, Button } from '@lumik/ui';
import { useBackupSession } from '../lib/BackupSessionContext';

const formStyles: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: '14px',
};
const rowStyles: React.CSSProperties = {
  display: 'flex',
  gap: '12px',
  alignItems: 'center',
  flexWrap: 'wrap',
};
const errorStyles: React.CSSProperties = {
  fontSize: '13px',
  color: 'var(--lumik-error, #ffb4ab)',
};

type Mode = 'login' | 'register';

interface BackupAccountFormProps {
  /** Se llama tras iniciar sesión o crear la cuenta con éxito. */
  onSignedIn?: () => void;
  /** Se espera antes de autenticar, p. ej. para guardar una URL del servidor
   *  editada y sin guardar. Si falla, se muestra el error y no se autentica. */
  beforeSubmit?: () => Promise<void>;
}

/** Formulario de inicio de sesión / registro en el servidor de respaldo. Usa la
 *  URL del servidor configurada en Ajustes → Respaldo. */
export function BackupAccountForm({ onSignedIn, beforeSubmit }: BackupAccountFormProps) {
  const { t } = useTranslation();
  const { login, register } = useBackupSession();
  const [mode, setMode] = useState<Mode>('login');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [name, setName] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async () => {
    setError(null);
    if (!email || !password) {
      setError(t('settings.backup.errors.required'));
      return;
    }
    setBusy(true);
    try {
      await beforeSubmit?.();
      if (mode === 'login') await login(email, password);
      else await register(email, password, name || undefined);
      setPassword('');
      onSignedIn?.();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <form
      style={formStyles}
      onSubmit={(e) => {
        e.preventDefault();
        submit();
      }}
    >
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
        <Button variant="primary" type="submit" disabled={busy}>
          {mode === 'login' ? t('settings.backup.login') : t('settings.backup.register')}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          type="button"
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
    </form>
  );
}
