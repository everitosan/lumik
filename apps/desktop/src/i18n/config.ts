import i18n from 'i18next';
import '@lumik/ui/i18n';
import es from './locales/es.json';
import en from './locales/en.json';

// Add desktop-specific translations to UI translations
i18n.addResourceBundle('es', 'translation', es, true, true);
i18n.addResourceBundle('en', 'translation', en, true, true);

export default i18n;
