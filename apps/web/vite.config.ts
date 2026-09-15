import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

// El dashboard web consume el server de respaldo (apps/server). En dev, todo lo
// que empiece con /v1 se hace proxy al server en :8787.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/v1': 'http://localhost:8787',
    },
  },
  resolve: {
    alias: {
      '@lumik/ui': path.resolve(__dirname, '../../packages/ui/src'),
    },
    // Una sola copia de estas libs (el @lumik/ui aliaseado trae las suyas).
    dedupe: ['i18next', 'react-i18next', 'react', 'react-dom'],
  },
  optimizeDeps: {
    exclude: ['@lumik/ui'],
  },
});
