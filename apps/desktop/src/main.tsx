import { StrictMode } from 'react';
import { LogoSplash } from '@lumik/ui';
import { createRoot } from 'react-dom/client';
import '@lumik/ui/tokens.css';
import './styles/global.css';
import './i18n/config';
import App from './App';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
    {/* Splash de arranque (spec `app-splash`): encima de la app, que carga debajo, y se quita solo. */}
    <LogoSplash />
  </StrictMode>
);
