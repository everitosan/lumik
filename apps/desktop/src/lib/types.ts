// Types matching Rust models

export interface Photographer {
  id: string;
  email: string;
  alias: string;
  active: boolean;
  deleted: boolean;
}

export interface Project {
  id: string;
  name: string;
  description: string | null;
  created_at: string;
  session_date: string | null;
  archived: boolean;
  deleted: boolean;
  creator_id: string;
  cover_photo_path: string | null;
}

// Project view for dashboard
export interface ProjectDashboard {
  id: string;
  name: string;
  created_at: string;
  session_date: string | null;
  photo_count: number;
  workflow_status: WorkflowStatus;
  cover_photo_path: string | null;
  device_uuid: string;
}

export type WorkflowStatus = 'imported' | 'edited' | 'delivered';

export type BackupStatus = 'pending' | 'uploaded' | 'failed';

export interface CreateProject {
  name: string;
  description?: string;
  session_date?: string;
  creator_id: string;
  /** UUID of the external device where the project.db will be created */
  device_uuid: string;
}

/** Device previously seen and stored in the global registry */
export interface KnownDevice {
  uuid: string;
  name: string;
  mount_hint: string | null;
  last_seen: string;
  registered_at: string;
}

// Detected device from system scan (runtime only, not stored in DB)
export interface DetectedDevice {
  uuid: string;
  name: string;
  mount_point: string;
  total_bytes: number | null;
  available_bytes: number | null;
  fs_type: string;
}

// Photographer metadata for XMP embedding
export interface PhotographerMetadata {
  id: string;
  photographer_id: string;
  artist: string | null;
  copyright: string | null;
  contact_email: string | null;
  contact_url: string | null;
  contact_phone: string | null;
  usage_terms: string | null;
  custom_tags: string | null;
}

export interface UpdatePhotographerMetadata {
  artist?: string;
  copyright?: string;
  contact_url?: string;
}

// Keybinding
export interface Keybinding {
  action: string;
  key: string;
  description: string;
}

/** Map de action → key para lookup rápido en handlers de teclado */
export type KeybindingMap = Record<string, string>;

// Application settings
export interface AppSettings {
  embed_metadata_on_import: boolean;
  rename_on_import: boolean;
}

// Per-project UI and workflow settings
export interface ProjectSettings {
  sidebar_open: boolean;
  show_culled: boolean;
  min_stars?: number | null;
  selected_tags?: string | null;
  selected_colors?: string | null;
  stars_filter_mode?: 'exact' | 'inclusive';
  view_mode?: 'grid' | 'by-date';
}

// ============================================================================
// IMPORT TYPES
// ============================================================================

export interface ImportRequest {
  session_id: string;
  source_files: string[];
  project_id: string;
  device_uuid: string;
  mount_point: string;
  project_name: string;
}

export type ImportPhase =
  | 'reading'
  | 'decoding'
  | 'converting'
  | 'hashing'
  | 'writing'
  | 'saving'
  | 'complete'
  | 'failed';

export interface ImportProgress {
  session_id: string;
  current_index: number;
  total_files: number;
  current_file: string;
  phase: ImportPhase;
  error: string | null;
}

export interface FailedFile {
  name: string;
  path: string;
  error: string;
}

export interface ImportResult {
  session_id: string;
  total_files: number;
  successful: number;
  failed: number;
  failed_files: FailedFile[];
  videos_copied: number;
}

export interface ImportLogEntry {
  session_id: string;
  message: string;
}

// ============================================================================
// PHOTO TYPE
// ============================================================================

export interface Photo {
  id: string;
  project_id: string;
  dng_path: string;
  jpg_path: string | null;
  device_uuid: string;
  original_camera: string | null;
  original_format: string | null;
  import_date: string;
  file_hash: string | null;
  culled: boolean;
  workflow_status: WorkflowStatus;
  backup_status: BackupStatus;
  backup_url: string | null;
  backup_date: string | null;
  backup_retries: number;
  deleted: boolean;
  stars: number;
  color_label: string | null;
  tags: string | null;
  capture_date: string | null;
  width: number | null;
  height: number | null;
  file_size_bytes: number | null;
  iso: number | null;
  aperture: string | null;
  shutter_speed: string | null;
  exposure_compensation: number | null;
  focal_length: string | null;
  lens_model: string | null;
  rotation: number;
}
