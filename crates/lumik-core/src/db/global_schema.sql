-- LUMIK - Global Database Schema
-- Stores app configuration, photographer profile, and known device registry.
-- Lives at: ~/.local/share/com.lumik.app/lumik.db


-- ============================================================================
-- PHOTOGRAPHER (local user)
-- ============================================================================

CREATE TABLE IF NOT EXISTS photographer (
    id              TEXT PRIMARY KEY,       -- UUID
    email           TEXT NOT NULL UNIQUE,
    alias           TEXT NOT NULL CHECK(length(alias) <= 10),
    active          INTEGER NOT NULL DEFAULT 1,
    deleted         INTEGER NOT NULL DEFAULT 0
);


-- ============================================================================
-- PHOTOGRAPHER METADATA (for XMP writing)
-- ============================================================================

CREATE TABLE IF NOT EXISTS photographer_metadata (
    id              TEXT PRIMARY KEY,       -- UUID
    photographer_id TEXT NOT NULL REFERENCES photographer(id),
    artist          TEXT,                   -- XMP:Artist
    copyright       TEXT,                   -- XMP:Copyright
    contact_email   TEXT,                   -- XMP:CreatorContactInfo.CiEmailWork
    contact_url     TEXT,                   -- XMP:CreatorContactInfo.CiUrlWork
    contact_phone   TEXT,                   -- XMP:CreatorContactInfo.CiTelWork
    usage_terms     TEXT,                   -- XMP:UsageTerms
    custom_tags     TEXT,                   -- JSON object
    UNIQUE(photographer_id)
);


-- ============================================================================
-- SETTINGS (application configuration)
-- ============================================================================

CREATE TABLE IF NOT EXISTS settings (
    key             TEXT PRIMARY KEY,
    value           TEXT NOT NULL
);


-- ============================================================================
-- KEYBINDINGS (configurable keyboard shortcuts)
-- ============================================================================

CREATE TABLE IF NOT EXISTS keybinding (
    action      TEXT PRIMARY KEY,  -- e.g. "photo_detail.zoom_in"
    key         TEXT NOT NULL,     -- value of KeyboardEvent.key
    description TEXT NOT NULL
);


-- ============================================================================
-- DEVICE REGISTRY
-- Known external drives. Populated when a device is first seen during a scan.
-- mount_hint is informational only — actual mount point is resolved at runtime.
-- ============================================================================

CREATE TABLE IF NOT EXISTS device (
    uuid            TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    mount_hint      TEXT,                   -- last known mount path (hint only)
    last_seen       TEXT NOT NULL,          -- ISO 8601 datetime
    registered_at   TEXT NOT NULL           -- ISO 8601 datetime
);


-- ============================================================================
-- BACKUP QUEUE (cola offline-first)
-- Intención de respaldo por proyecto: se persiste al iniciar un respaldo y se
-- borra al completarse. Al arrancar la app se reanudan los pendientes.
-- ============================================================================

CREATE TABLE IF NOT EXISTS backup_queue (
    project_id  TEXT PRIMARY KEY,
    targets     TEXT,                       -- csv de proveedores (o NULL = todos)
    quality     TEXT,                       -- original/high/medium/low
    created_at  TEXT NOT NULL               -- ISO 8601 datetime
);


-- ============================================================================
-- SEARCH INDEX (spec photo-search)
-- Copia de las fotos de todos los proyectos vistos por este equipo, para buscar
-- aunque su disco no esté conectado. Desechable: se reconstruye desde cada
-- project.db al conectar los discos. Las columnas *_norm van en minúsculas y
-- sin acentos (domain::search::normalize).
-- ============================================================================

CREATE TABLE IF NOT EXISTS search_project (
    project_id          TEXT PRIMARY KEY,
    name                TEXT NOT NULL,
    description         TEXT,
    session_date        TEXT,
    created_at          TEXT NOT NULL,
    archived            INTEGER NOT NULL DEFAULT 0,
    device_uuid         TEXT,                   -- último disco donde se vio (para "conecta…")
    device_name         TEXT,
    name_norm           TEXT NOT NULL DEFAULT '',
    text_norm           TEXT NOT NULL DEFAULT '', -- nombre + descripción
    indexed_generation  INTEGER NOT NULL DEFAULT -1,
    indexed_at          TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS search_photo (
    photo_id        TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES search_project(project_id) ON DELETE CASCADE,
    file_name       TEXT NOT NULL,
    capture_date    TEXT,                       -- 'AAAA-MM-DD HH:MM:SS' (comparable como texto)
    stars           INTEGER NOT NULL DEFAULT 0,
    colors          TEXT NOT NULL DEFAULT ',',  -- ',1,3,'
    tags            TEXT,                       -- como los ve el usuario: 'calle,retrato'
    tags_norm       TEXT NOT NULL DEFAULT ',',  -- ',calle,retrato,'
    text_norm       TEXT NOT NULL DEFAULT '',   -- tags + nombre de archivo
    culled          INTEGER NOT NULL DEFAULT 0,
    delivered       INTEGER NOT NULL DEFAULT 0,
    is_pair         INTEGER NOT NULL DEFAULT 0,
    camera          TEXT,
    camera_norm     TEXT NOT NULL DEFAULT '',
    lens            TEXT,
    lens_norm       TEXT NOT NULL DEFAULT '',
    rotation        INTEGER NOT NULL DEFAULT 0, -- con la que se hizo `thumb`
    thumb           BLOB,                       -- JPEG de 128 px para resultados sin disco
    embedding       BLOB                        -- vector de la foto (semantic-search); NULL por ahora
);

CREATE INDEX IF NOT EXISTS idx_search_photo_project ON search_photo(project_id);
CREATE INDEX IF NOT EXISTS idx_search_photo_date ON search_photo(capture_date);
