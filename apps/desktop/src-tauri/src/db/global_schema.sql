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
