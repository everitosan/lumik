import { useEffect, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Button, Input } from '@lumik/ui';
import { providersApi, type Remote } from '../api/client';

type CredentialProvider = 'nextcloud' | 'mega';

export function ProvidersPanel() {
  const { t } = useTranslation();
  const [remotes, setRemotes] = useState<Remote[]>([]);
  const [type, setType] = useState<CredentialProvider>('nextcloud');
  const [fields, setFields] = useState<Record<string, string>>({});
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = () => {
    providersApi.list().then(setRemotes).catch(() => setRemotes([]));
  };
  useEffect(refresh, []);

  const setField = (k: string, v: string) => setFields((f) => ({ ...f, [k]: v }));

  const onSelectType = (next: CredentialProvider) => {
    setType(next);
    setFields({});
    setError(null);
  };

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      await providersApi.connect(type, fields);
      setFields({});
      refresh();
    } catch {
      setError(t('providers.error'));
    } finally {
      setBusy(false);
    }
  };

  const disconnect = async (id: string) => {
    await providersApi.disconnect(id).catch(() => undefined);
    refresh();
  };

  return (
    <section className="providers">
      <div>
        <h2>{t('providers.title')}</h2>
        <p className="subtitle">{t('providers.subtitle')}</p>
      </div>

      {/* Conectados */}
      <div className="providers-list">
        <h3>{t('providers.connected')}</h3>
        {remotes.length === 0 ? (
          <p className="subtitle">{t('providers.none')}</p>
        ) : (
          remotes.map((r) => (
            <div key={r.id} className="provider-row">
              <span>{t(`providers.types.${r.provider}`, { defaultValue: r.provider })}</span>
              <Button variant="ghost" size="sm" onClick={() => disconnect(r.id)}>
                {t('providers.disconnect')}
              </Button>
            </div>
          ))
        )}
      </div>

      {/* Conectar nuevo */}
      <form className="auth-form" onSubmit={onSubmit}>
        <h3>{t('providers.connect')}</h3>
        <label className="providers-type">
          <span>{t('providers.type')}</span>
          <select
            value={type}
            onChange={(e) => onSelectType(e.target.value as CredentialProvider)}
          >
            <option value="nextcloud">{t('providers.types.nextcloud')}</option>
            <option value="mega">{t('providers.types.mega')}</option>
          </select>
        </label>

        {type === 'nextcloud' && (
          <>
            <Input
              label={t('providers.fields.url')}
              value={fields.url ?? ''}
              onChange={(e) => setField('url', e.target.value)}
              fullWidth
            />
            <Input
              label={t('providers.fields.username')}
              value={fields.username ?? ''}
              onChange={(e) => setField('username', e.target.value)}
              fullWidth
            />
          </>
        )}
        {type === 'mega' && (
          <Input
            label={t('providers.fields.email')}
            type="email"
            value={fields.email ?? ''}
            onChange={(e) => setField('email', e.target.value)}
            fullWidth
          />
        )}
        <Input
          label={t('providers.fields.password')}
          type="password"
          value={fields.password ?? ''}
          onChange={(e) => setField('password', e.target.value)}
          fullWidth
        />
        {error && <span className="auth-error">{error}</span>}
        <Button type="submit" variant="primary" disabled={busy}>
          {t('providers.submit')}
        </Button>
        <p className="subtitle">{t('providers.oauthComingSoon')}</p>
      </form>
    </section>
  );
}
