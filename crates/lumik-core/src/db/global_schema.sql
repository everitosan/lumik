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
    exports_signature   TEXT NOT NULL DEFAULT '', -- firma de _exported/ al indexar
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
    lens_norm       TEXT NOT NULL DEFAULT ''
);

-- Miniatura y vector de cada foto, aparte de search_photo para que las
-- búsquedas recorran solo filas chicas (con las miniaturas en la misma fila,
-- 70 000 fotos son ~300 MB de páginas por búsqueda).
CREATE TABLE IF NOT EXISTS search_media (
    photo_id        TEXT PRIMARY KEY REFERENCES search_photo(photo_id) ON DELETE CASCADE,
    rotation        INTEGER NOT NULL DEFAULT 0, -- con la que se hizo `thumb`
    thumb           BLOB,                       -- JPEG de 128 px para resultados sin disco
    embedding       BLOB                        -- vector de la foto (semantic-search); NULL por ahora
);

-- Archivos de _exported/ de cada proyecto (finales), con su foto si están
-- ligados por nombre. export_id = uuid5(project_id/nombre), el mismo del
-- respaldo y del portal. signature = "tamaño:fecha" para rehacer la miniatura
-- solo si el archivo cambió.
CREATE TABLE IF NOT EXISTS search_export (
    export_id       TEXT PRIMARY KEY,
    project_id      TEXT NOT NULL REFERENCES search_project(project_id) ON DELETE CASCADE,
    file_name       TEXT NOT NULL,
    photo_id        TEXT,
    signature       TEXT NOT NULL,
    thumb           BLOB
);

CREATE INDEX IF NOT EXISTS idx_search_export_project ON search_export(project_id);
CREATE INDEX IF NOT EXISTS idx_search_export_photo ON search_export(photo_id);

CREATE INDEX IF NOT EXISTS idx_search_photo_project ON search_photo(project_id);
CREATE INDEX IF NOT EXISTS idx_search_photo_date ON search_photo(capture_date);


-- ============================================================================
-- ALBUMS (spec photo-albums)
-- Álbumes de curación del equipo con fotos de varios proyectos y discos. Un
-- elemento referencia una foto (kind 'photo', ref_id = photo.id) o un archivo
-- de _exported/ (kind 'export', ref_id = export_id). file_name y project_name
-- son copias para mostrar el elemento si su foto ya no existe.
-- ============================================================================

CREATE TABLE IF NOT EXISTS album (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS album_item (
    id           TEXT PRIMARY KEY,
    album_id     TEXT NOT NULL REFERENCES album(id) ON DELETE CASCADE,
    kind         TEXT NOT NULL CHECK(kind IN ('photo', 'export')),
    project_id   TEXT NOT NULL,
    ref_id       TEXT NOT NULL,
    file_name    TEXT NOT NULL,
    project_name TEXT NOT NULL,
    added_at     TEXT NOT NULL,
    UNIQUE(album_id, kind, ref_id)
);

CREATE INDEX IF NOT EXISTS idx_album_item_album ON album_item(album_id);
