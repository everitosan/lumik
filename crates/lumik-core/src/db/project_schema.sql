-- LUMIK - Per-Project Database Schema
-- One file per project, lives at: {mount}/lumik/{project_slug}/project.db
-- Travels with the external drive — no dependency on the global database.


-- ============================================================================
-- PROJECT (single row per database)
-- creator_id references global photographer.id — no FK enforced across DBs.
-- ============================================================================

CREATE TABLE IF NOT EXISTS project (
    id              TEXT PRIMARY KEY,       -- UUID, stable across machines
    name            TEXT NOT NULL,
    description     TEXT,
    created_at      TEXT NOT NULL,          -- ISO 8601 datetime
    session_date    TEXT,                   -- Photo session date (for sorting)
    archived        INTEGER NOT NULL DEFAULT 0,
    deleted         INTEGER NOT NULL DEFAULT 0,
    creator_id      TEXT NOT NULL,          -- references global photographer.id (no FK)
    cover_photo_path TEXT,                  -- relative path to thumbnail, e.g. ".thumbs/{photo_id}.jpg"
    generation      INTEGER NOT NULL DEFAULT 0  -- sube con cada escritura; se copia al candado (.lumik-lock)
);

CREATE INDEX IF NOT EXISTS idx_project_date ON project(session_date) WHERE deleted = 0;


-- ============================================================================
-- PHOTO
-- ============================================================================

CREATE TABLE IF NOT EXISTS photo (
    id              TEXT PRIMARY KEY,       -- UUID

    -- Relationship with project (same DB)
    project_id      TEXT NOT NULL REFERENCES project(id),

    -- File location
    dng_path        TEXT NOT NULL,          -- Path RELATIVE to the project folder (e.g. "_media/IMG.dng")
    jpg_path        TEXT,                   -- Edited JPG, relative to the project folder (nullable)

    -- OBSOLETO: UUID del disco donde se importó. La identidad portable es
    -- project.id y el disco se resuelve en runtime (scan_mounted_devices).
    -- Las fotos nuevas guardan ''; las viejas conservan su valor, que no se lee.
    -- Se conserva la columna para no migrar los project.db de los discos.
    device_uuid     TEXT NOT NULL,

    -- Source data
    original_camera TEXT,
    original_format TEXT,
    import_date     TEXT NOT NULL,          -- ISO 8601 datetime

    -- Integrity
    file_hash       TEXT,                   -- SHA-256 of DNG file (reserved, not computed on import)

    -- Culling
    culled          INTEGER NOT NULL DEFAULT 0,  -- If true, file lives in _culled/

    -- Workflow
    workflow_status TEXT NOT NULL DEFAULT 'imported'
                    CHECK(workflow_status IN ('imported', 'edited', 'delivered')),

    -- Backup via API
    backup_status   TEXT NOT NULL DEFAULT 'pending'
                    CHECK(backup_status IN ('pending', 'uploaded', 'failed')),
    backup_url      TEXT,
    backup_date     TEXT,
    backup_retries  INTEGER NOT NULL DEFAULT 0,

    -- Soft delete
    deleted         INTEGER NOT NULL DEFAULT 0,

    -- DNG metadata cache (source of truth is the file, this is for fast queries)
    stars           INTEGER NOT NULL DEFAULT 0 CHECK(stars BETWEEN 0 AND 5),
    color_label     TEXT,
    tags            TEXT,                   -- JSON array of XMP Subject tags
    capture_date    TEXT,                   -- EXIF DateTimeOriginal
    width           INTEGER,
    height          INTEGER,
    file_size_bytes INTEGER,

    -- EXIF camera parameters cache
    iso                   INTEGER,          -- e.g. 100
    aperture              TEXT,             -- e.g. "f/2.8"
    shutter_speed         TEXT,             -- e.g. "1/500"
    exposure_compensation REAL,             -- EV compensation, e.g. 0.0
    focal_length          TEXT,             -- e.g. "35 mm"
    lens_model            TEXT,             -- e.g. "SUMMILUX-M 35MM f/1.4 ASPH"

    -- Orientation from EXIF, cached so the canvas never reads the file
    rotation              INTEGER NOT NULL DEFAULT 0, -- degrees: 0 / 90 / 180 / 270

    -- RAW+JPEG pair: the camera JPG of the same shot (dng_path is the RAW).
    -- Relative to the project folder; NULL if the photo is not a pair.
    camera_jpg_path       TEXT,
    -- Camera file name without extension (e.g. "DSCF1234"), kept to pair a
    -- file that arrives later with a photo whose files were already renamed.
    source_stem           TEXT,

    -- Camera JPG delivered to _exported/ on approval (camera-jpeg-delivery):
    -- fingerprint of the delivered file and how to undo the delivery
    -- ('moved' = the JPG of a pair, 'copied' = copy of a standalone JPG).
    delivered_size        INTEGER,
    delivered_sha256      TEXT,
    delivery_kind         TEXT CHECK(delivery_kind IN ('moved', 'copied'))
);

CREATE INDEX IF NOT EXISTS idx_photo_project  ON photo(project_id) WHERE deleted = 0;
-- OBSOLETO junto con photo.device_uuid; se conserva para no migrar los project.db.
CREATE INDEX IF NOT EXISTS idx_photo_device   ON photo(device_uuid);
CREATE INDEX IF NOT EXISTS idx_photo_culled   ON photo(project_id, culled) WHERE deleted = 0;
CREATE INDEX IF NOT EXISTS idx_photo_stars    ON photo(project_id, stars) WHERE deleted = 0;
CREATE INDEX IF NOT EXISTS idx_photo_color    ON photo(project_id, color_label) WHERE deleted = 0;
CREATE INDEX IF NOT EXISTS idx_photo_backup   ON photo(backup_status) WHERE deleted = 0;
CREATE INDEX IF NOT EXISTS idx_photo_workflow ON photo(project_id, workflow_status) WHERE deleted = 0;


-- ============================================================================
-- PROJECT SETTINGS (single row per database, id always = 1)
-- UI and workflow preferences that persist per project.
-- Add new settings as columns with DEFAULT so existing DBs migrate automatically.
-- ============================================================================

CREATE TABLE IF NOT EXISTS project_settings (
    id                  INTEGER PRIMARY KEY DEFAULT 1 CHECK(id = 1),
    sidebar_open        INTEGER NOT NULL DEFAULT 1,   -- 1 = metadata panel open in PhotoDetail
    show_culled         INTEGER NOT NULL DEFAULT 0,   -- 1 = show culled filter active in ProjectDetail
    min_stars           INTEGER,                      -- minimum rating filter (1-5, or NULL for no filter)
    selected_tags       TEXT,                         -- comma-separated tags (or NULL)
    selected_colors     TEXT,                         -- comma-separated color labels 1-5 (or NULL)
    stars_filter_mode   TEXT,                         -- 'exact' | 'inclusive' (or NULL for default)
    view_mode           TEXT,                         -- 'grid' | 'by-date' (or NULL for default 'by-date')
    -- What approving delivers straight to _exported/ (camera-jpeg-delivery).
    delivery_mode       TEXT NOT NULL DEFAULT 'none'
                        CHECK(delivery_mode IN ('none', 'pairs', 'all_camera_jpg')),
    -- When photos get their content vector (semantic-search).
    analysis_mode       TEXT NOT NULL DEFAULT 'on_demand'
                        CHECK(analysis_mode IN ('on_import', 'on_demand'))
);

-- ============================================================================
-- PHOTO EMBEDDINGS (semantic-search)
-- Content vector of each photo, computed from its thumbnail. Lives on the disk
-- so it travels with the project; `model` and `rotation` tell whether it is
-- still valid (another model or a rotated photo means it must be recomputed).
-- ============================================================================

CREATE TABLE IF NOT EXISTS photo_embedding (
    photo_id    TEXT PRIMARY KEY REFERENCES photo(id) ON DELETE CASCADE,
    model       TEXT NOT NULL,
    rotation    INTEGER NOT NULL,
    vector      BLOB NOT NULL      -- f32 little-endian, normalized
);
