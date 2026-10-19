# Lumik Server (respaldo)

Backend SaaS multi-tenant para el respaldo de `_exported` a la nube. Stack: **Rust
+ axum + sqlx (Postgres)**, con **rclone** como backend de subida.

Plan y decisiones: [../../docs/backup-plan.md](../../docs/backup-plan.md).

## Estado — beta (fases 0–9 del plan)

- **Infra**: `GET /healthz` (verifica la BD), Postgres vía `docker-compose`,
  migraciones al arrancar y rclone como backend de subida.
- **Cuentas** (`/v1/auth/*`): registro, login, refresh con rotación, logout y `me`.
  Contraseñas argon2id, JWT de acceso + refresh, rate limit por IP y aislamiento por
  fotógrafo.
- **Proveedores** (`/v1/providers`): Nextcloud y Mega por credenciales (cifradas con
  AES-256-GCM). Google Drive, Dropbox y OneDrive responden 501 hasta registrar sus apps
  OAuth.
- **Catálogo** (`/v1/sync/*`, `/v1/projects*`): el desktop publica proyectos, fotos y
  portada; los clientes listan proyectos y fotos con su estado de respaldo.
- **Respaldo** (`/v1/projects/:id/backup/*`, `.../photos/:photo_id/backup`): subida
  con fan-out a varios proveedores, recompresión opcional, SHA-256 + tamaño, carpeta
  fija por proyecto (`prepare`), traslado al renombrar (`relocate`) y enlace con
  caducidad a la carpeta (`link`).

Comportamiento esperado: [`openspec/specs/`](../../openspec/specs/) (`backup-accounts`,
`backup-providers`, `catalog-sync`, `cloud-backup`).

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

También se puede levantar desde la raíz con `make dev-server` (Postgres + API).

## Esquema del catálogo (`migrations/`)

`photographer` (cuenta) · `refresh_token` · `project` (con portada, estado de flujo y
`backup_dir`) · `photo` (con `backup_*`, hash y tamaño) · `remote` (credenciales de
nube por fotógrafo, cifradas). Todo scoped por `photographer_id`.
