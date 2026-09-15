# Lumik Web (portal de respaldo)

Portal web (React 19 + Vite + `@lumik/ui`) para el respaldo de sesiones. Consume
el server de respaldo (`apps/server`).

Plan: [../../docs/backup-plan.md](../../docs/backup-plan.md).

## Estado — Fase 2 (login en web)

- Registro / inicio de sesión / cierre de sesión contra `/v1/auth/*`.
- Sesión persistente (tokens en `localStorage`) con **refresh transparente en 401**.
- Ruta protegida de ejemplo (`/`) que consume `GET /v1/auth/me`.

El dashboard de proyectos/respaldos llega en fases posteriores.

## Dev

```bash
# 1. Server de respaldo corriendo (otra terminal)
cd apps/server && docker compose up -d && cargo run

# 2. Portal web (proxy /v1 → :8787)
pnpm --filter @lumik/web dev      # http://localhost:5173
```
