# Lumik Web (portal de respaldo)

Portal web (React 19 + Vite + `@lumik/ui`) para el respaldo de sesiones. Consume
el server de respaldo (`apps/server`).

Plan: [../../docs/backup-plan.md](../../docs/backup-plan.md).

## Estado — beta (fases 2, 3 y 8 del plan)

- **Cuenta**: registro, inicio y cierre de sesión contra `/v1/auth/*`. Sesión en
  `localStorage` con **refresh transparente en 401**.
- **Dashboard (`/`)**: proyectos del catálogo agrupados por año, con portada, fotos
  respaldadas, estado e iconos de proveedor. El icono abre la carpeta en la nube
  (enlace con caducidad en Dropbox/OneDrive) o la web del proveedor.
- **Subida desde el navegador / iPad**: al abrir un proyecto se eligen fotos,
  proveedores y calidad; 6 subidas en paralelo con reintentos y sin duplicar lo ya
  respaldado (mismo id determinista que el desktop).
- **Configuración (`/config`)**: conectar/desconectar Nextcloud y Mega. Los
  proveedores OAuth (Google Drive, Dropbox, OneDrive) aún no están disponibles.

Comportamiento esperado: [`openspec/specs/web-portal`](../../openspec/specs/web-portal/spec.md).

## Dev

```bash
# Todo junto desde la raíz (Postgres + API :8787 + web :5173)
make dev-web

# O por separado:
# 1. Server de respaldo corriendo (otra terminal)
cd apps/server && docker compose up -d && cargo run

# 2. Portal web (proxy /v1 → :8787)
pnpm --filter @lumik/web dev      # http://localhost:5173
```
