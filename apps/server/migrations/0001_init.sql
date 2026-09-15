-- Catálogo multi-tenant del server de respaldo Lumik.
-- Todo cuelga de `photographer` (misma entidad que en la app; en el server gana
-- email/password_hash). Ver docs/backup-plan.md §4 y §6.

-- ============================================================================
-- PHOTOGRAPHER  (cuenta SaaS + identidad; id === creator_id local)
-- ============================================================================
CREATE TABLE IF NOT EXISTS photographer (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           TEXT NOT NULL UNIQUE,
    password_hash   TEXT NOT NULL,          -- argon2id (Fase 1)
    name            TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ============================================================================
-- PROJECT  (espejo ligero del proyecto local; metadata la manda la desktop)
-- ============================================================================
CREATE TABLE IF NOT EXISTS project (
    id              UUID PRIMARY KEY,       -- === project.id local
    photographer_id UUID NOT NULL REFERENCES photographer(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    session_date    DATE,
    slug            TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_project_photographer ON project(photographer_id);

-- ============================================================================
-- PHOTO  (candidatas de _exported; el estado de backup lo manda el server)
-- ============================================================================
CREATE TABLE IF NOT EXISTS photo (
    id              UUID PRIMARY KEY,       -- === photo.id local
    project_id      UUID NOT NULL REFERENCES project(id) ON DELETE CASCADE,
    filename        TEXT NOT NULL,
    file_hash       TEXT,
    jpg_path        TEXT,
    backup_status   TEXT NOT NULL DEFAULT 'pending'
                    CHECK (backup_status IN ('pending', 'uploaded', 'failed')),
    backup_url      TEXT,
    backup_date     TIMESTAMPTZ,
    backup_retries  INTEGER NOT NULL DEFAULT 0,
    provider        TEXT,                   -- proveedor(es) usados (agregado en MVP)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_photo_project ON photo(project_id);
CREATE INDEX IF NOT EXISTS idx_photo_backup  ON photo(backup_status);

-- ============================================================================
-- REMOTE  (credenciales de nube por fotógrafo; token cifrado en Fase 3)
-- ============================================================================
CREATE TABLE IF NOT EXISTS remote (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    photographer_id UUID NOT NULL REFERENCES photographer(id) ON DELETE CASCADE,
    provider        TEXT NOT NULL
                    CHECK (provider IN ('gdrive', 'dropbox', 'onedrive', 'mega', 'nextcloud')),
    name            TEXT NOT NULL,          -- nombre del remote rclone, p.ej. u{id}_gdrive
    token           TEXT,                   -- token/credencial cifrada (Fase 3)
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (photographer_id, provider)
);

CREATE INDEX IF NOT EXISTS idx_remote_photographer ON remote(photographer_id);
