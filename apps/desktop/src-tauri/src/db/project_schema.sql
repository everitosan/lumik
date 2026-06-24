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
    cover_photo_path TEXT                   -- relative path to thumbnail, e.g. ".thumbs/{photo_id}.jpg"
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
    dng_path        TEXT NOT NULL,          -- Path RELATIVE to device mount point
    jpg_path        TEXT,                   -- Relative path to edited JPG (nullable)

    -- Storage device UUID (resolved at runtime via scan_mounted_devices)
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
    rotation              INTEGER NOT NULL DEFAULT 0  -- degrees: 0 / 90 / 180 / 270
);

CREATE INDEX IF NOT EXISTS idx_photo_project  ON photo(project_id) WHERE deleted = 0;
CREATE INDEX IF NOT EXISTS idx_photo_device   ON photo(device_uuid);
CREATE INDEX IF NOT EXISTS idx_photo_culled   ON photo(project_id, culled) WHERE deleted = 0;
CREATE INDEX IF NOT EXISTS idx_photo_stars    ON photo(project_id, stars) WHERE deleted = 0;
CREATE INDEX IF NOT EXISTS idx_photo_color    ON photo(project_id, color_label) WHERE deleted = 0;
CREATE INDEX IF NOT EXISTS idx_photo_backup   ON photo(backup_status) WHERE deleted = 0;
CREATE INDEX IF NOT EXISTS idx_photo_workflow ON photo(project_id, workflow_status) WHERE deleted = 0;
