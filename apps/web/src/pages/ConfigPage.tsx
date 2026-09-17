import { useTranslation } from 'react-i18next';
import { ProvidersPanel } from '../components/ProvidersPanel';

export default function ConfigPage() {
  const { t } = useTranslation();
  return (
    <div className="page">
      <h1>{t('nav.config')}</h1>
      <ProvidersPanel />
    </div>
  );
}
