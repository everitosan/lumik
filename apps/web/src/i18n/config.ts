import i18n from 'i18next';
// Inicializa la instancia compartida con los bundles de @lumik/ui.
import '@lumik/ui/i18n';
import es from './locales/es.json';
import en from './locales/en.json';

// Añade las traducciones del portal web sobre las de la UI.
i18n.addResourceBundle('es', 'translation', es, true, true);
i18n.addResourceBundle('en', 'translation', en, true, true);

export default i18n;
