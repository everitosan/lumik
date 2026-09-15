import { useTranslation } from 'react-i18next';
import { Button } from '@lumik/ui';
import { useAuth } from '../auth/AuthContext';

export default function Dashboard() {
  const { t } = useTranslation();
  const { photographer, logout } = useAuth();

  return (
    <div className="dashboard">
      <div className="dashboard-header">
        <h1>{t('dashboard.title')}</h1>
        <Button variant="secondary" size="sm" onClick={logout}>
          {t('dashboard.logout')}
        </Button>
      </div>
      <div>
        <p>{t('dashboard.welcome', { name: photographer?.name || photographer?.email })}</p>
        <p className="subtitle">
          {t('dashboard.signedInAs', { email: photographer?.email })}
        </p>
      </div>
      <p className="subtitle">{t('dashboard.empty')}</p>
    </div>
  );
}
