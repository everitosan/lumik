import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  resolve: {
    alias: {
      '@lumik/ui': path.resolve(__dirname, '../../packages/ui/src'),
    },
    // Force a single copy of i18next / react-i18next. The aliased @lumik/ui source
    // resolves these from its own deps, which otherwise pulls a second instance and
    // breaks i18n.addResourceBundle() in the production bundle.
    dedupe: ['i18next', 'react-i18next', 'react', 'react-dom'],
  },
  optimizeDeps: {
    exclude: ['@lumik/ui'],
  },
});
