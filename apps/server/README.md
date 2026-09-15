# Lumik Server (respaldo)

Backend SaaS multi-tenant para el respaldo de `_exported` a la nube. Stack: **Rust
+ axum + sqlx (Postgres)**, con **rclone** como backend de subida.

Plan y decisiones: [../../docs/backup-plan.md](../../docs/backup-plan.md).

## Estado — Fase 0 (infra)

- Server axum con `GET /healthz` (verifica la BD).
- Postgres (catálogo) vía `docker-compose`, con migraciones (`migrations/`).
- Integración con rclone + smoke test de subida.

Las capas de **auth**, **catálogo/sync** y **backup** se añaden en fases
posteriores (ver el plan).

## Puesta en marcha (dev)

```bash
cd apps/server

# 1. Base de datos
docker compose up -d

# 2. rclone (binario local, sin sudo)
./scripts/install_rclone.sh

# 3. Config
cp .env.example .env

# 4. Server (aplica migraciones al arrancar)
cargo run

# En otra terminal:
curl -s localhost:8787/healthz     # → {"status":"ok","db":"up"}

# 5. Smoke test de rclone (subida de un archivo vía rcd)
./scripts/rclone_smoke.sh          # → ✅ SMOKE OK
```

## Esquema del catálogo (`migrations/0001_init.sql`)

`photographer` (cuenta) · `project` · `photo` (con `backup_*`) · `remote`
(credenciales de nube por fotógrafo). Todo scoped por `photographer_id`.
