import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://lumik.evesan.rocks',
  i18n: {
    defaultLocale: 'en',
    locales: ['en', 'es'],
    routing: {
      prefixDefaultLocale: false,
    },
  },
});
