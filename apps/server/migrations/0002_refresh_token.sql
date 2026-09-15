-- Refresh tokens (rotación + revocación). Se guarda solo el hash del token
-- opaco (alta entropía), nunca el token en claro. Ver docs/backup-plan.md §4.

CREATE TABLE IF NOT EXISTS refresh_token (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    photographer_id UUID NOT NULL REFERENCES photographer(id) ON DELETE CASCADE,
    token_hash      TEXT NOT NULL UNIQUE,   -- sha256 hex del token opaco
    expires_at      TIMESTAMPTZ NOT NULL,
    revoked         BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_refresh_photographer ON refresh_token(photographer_id);
