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
  backup_providers: string | null;
  /** Archivado: oculto en el dashboard salvo que se muestren los archivados */
  archived: boolean;
  device_uuid: string;
  /** Otra instalación tiene el proyecto (candado); null si está libre o es esta. */
  locked_by: ProjectLockHolder | null;
}

/** Quién tiene un proyecto según su candado (spec project-lock). */
export interface ProjectLockHolder {
  name: string;
  /** Inicio de su sesión (RFC 3339). */
  since: string;
  /** Dejó cambios que no llegaron al disco. */
  pending: boolean;
}

/** Cómo se abrió la vista de un proyecto. */
export type ProjectAccess =
  | { access: 'open' }
  | { access: 'read_only'; holder: ProjectLockHolder };

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
  /** Mounted without write access: listed, but no projects are opened or created on it. */
  read_only: boolean;
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
  finder_tags_sidecar: boolean;
  /** Valor de fábrica de la entrega al aprobar para proyectos nuevos */
  default_delivery_mode: DeliveryMode;
  /** Cuándo se analizan las fotos de los proyectos nuevos (semantic-search) */
  default_analysis_mode: AnalysisMode;
}

/**
 * Estado del modelo de búsqueda por contenido en este equipo (semantic-search).
 * `error` es `código:detalle` de la última descarga fallida (`network`, `corrupt`, `io`).
 */
export type ContentModelState =
  | { state: 'not_installed'; size: number; error: string | null }
  | { state: 'downloading'; downloaded: number; total: number }
  | { state: 'installed'; size: number };

/** Cuándo se calculan los vectores de las fotos de un proyecto (semantic-search). */
export type AnalysisMode = 'on_import' | 'on_demand';

/**
 * "Al aprobar, entregar directamente": nada, el JPG de los pares RAW+JPG, o
 * todo JPG de cámara (pares y JPG sueltos). Cada proyecto guarda el suyo.
 */
export type DeliveryMode = 'none' | 'pairs' | 'all_camera_jpg';

export const DELIVERY_MODES: DeliveryMode[] = ['none', 'pairs', 'all_camera_jpg'];

/** Qué pasaría con las fotos aprobadas al cambiar la entrega (para el aviso) */
export interface DeliveryPlan {
  deliver: number;
  return_back: number;
  edits_kept: number;
  in_cloud: number;
}

export interface DeliveryChangeOutcome {
  delivered: number;
  returned: number;
  edits_kept: number;
  /** [id de foto, motivo] */
  failed: [string, string][];
}

// Orden del dashboard elegido por el usuario (BD global)
export type SortDirection = 'asc' | 'desc';

export interface DashboardSort {
  /** Secciones de año */
  years: SortDirection;
  /** Proyectos dentro de cada año */
  projects: SortDirection;
}

/** Orden histórico: años recientes primero, proyectos antiguos primero. */
export const DEFAULT_DASHBOARD_SORT: DashboardSort = { years: 'desc', projects: 'asc' };

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

/** Ajustes elegidos en el wizard solo para esta importación. */
export interface ImportSessionSettings {
  rename_on_import: boolean;
  embed_metadata: boolean;
  artist?: string;
  copyright?: string;
  contact_url?: string;
}

export interface ImportRequest {
  session_id: string;
  source_files: string[];
  project_id: string;
  device_uuid: string;
  mount_point: string;
  project_name: string;
  /** Sin ajustes, el backend usa la configuración global. */
  settings?: ImportSessionSettings;
}

/** Reintento del registro de fotos que ya están en `_media/` (fallaron con `register_failed`). */
export interface RetryRegisterRequest {
  session_id: string;
  project_id: string;
  project_name: string;
  /** Nombres en `_media/` (`FailedFile.name`). */
  file_names: string[];
}

export type ImportPhase =
  | 'videos'
  | 'copy'
  | 'copy_metadata'
  | 'metadata'
  | 'move'
  | 'register'
  | 'complete';

/** Evento `import-progress`. `file_index` empieza en 1; los bytes solo avanzan en `videos`, `copy` y `copy_metadata` (con varios archivos en paralelo, `file_*` es el último que empezó). */
export interface ImportProgress {
  session_id: string;
  phase: ImportPhase;
  file_name: string;
  file_index: number;
  file_total: number;
  file_bytes: number;
  file_bytes_total: number;
  bytes_done: number;
  bytes_total: number;
}

/** Motivo de un fallo; se traduce con `import.errors.codes.<code>`. */
export type ImportErrorCode =
  | 'source_unreadable'
  | 'no_space'
  | 'write_failed'
  | 'metadata_failed'
  | 'register_failed'
  | 'unknown';

export interface FailedFile {
  name: string;
  path: string;
  code: ImportErrorCode;
  /** Detalle técnico */
  error: string;
}

export type SkipReason = 'unsupported_format' | 'not_a_file';

export interface SkippedFile {
  name: string;
  path: string;
  reason: SkipReason;
}

/** Error que terminó el import antes de tiempo. */
export interface FatalError {
  code: ImportErrorCode;
  detail: string;
}

// Editar proyecto
export interface EditProjectRequest {
  id: string;
  name: string;
  /** AAAA-MM-DD, obligatoria */
  session_date: string;
  description?: string;
  /** Renombrar también las fotos importadas con el nombre anterior */
  rename_files: boolean;
  /** Reescribir {descripción}@{año} en las fotos ya importadas */
  rewrite_metadata: boolean;
}

export interface EditProjectResult {
  project: Project;
  renamed_files: number;
  metadata_updated: number;
  metadata_failed: FailedFile[];
}

export interface EditProjectProgress {
  project_id: string;
  done: number;
  total: number;
}

export interface ImportResult {
  session_id: string;
  total_files: number;
  successful: number;
  failed: number;
  failed_files: FailedFile[];
  skipped_files: SkippedFile[];
  videos_copied: number;
  /** Archivos que se unieron a una foto que ya estaba en el proyecto (la otra mitad de un par) */
  joined: number;
  fatal: FatalError | null;
}

/** Desglose por tomas de una selección (un par RAW+JPG cuenta como una foto) */
export interface ImportBreakdown {
  /** Fotos nuevas: pares + sueltos */
  photos: number;
  pairs: number;
  raw_only: number;
  jpg_only: number;
  other: number;
  /** Archivos que se unirán a una foto que ya está en el proyecto */
  joins: number;
}

export type ImportLogKind =
  | 'video_copied'
  | 'file_copied'
  | 'file_failed'
  | 'file_skipped'
  | 'photos_registered'
  | 'photos_joined'
  | 'thumbnails'
  | 'thumbnail'
  | 'folder_moved'
  | 'done';

/** Evento `import-log`; se traduce con `import.log.<kind>`. */
export interface ImportLogEntry {
  session_id: string;
  kind: ImportLogKind;
  file_name?: string;
  code?: ImportErrorCode;
  count?: number;
  detail?: string;
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
  /** JPG de cámara de un par RAW+JPG (`dng_path` es el RAW); null si no es par */
  camera_jpg_path: string | null;
  source_stem: string | null;
  /** Huella del JPG entregado a _exported/ al aprobar */
  delivered_size: number | null;
  delivered_sha256: string | null;
  delivery_kind: 'moved' | 'copied' | null;
}

// Sesión de la cuenta de respaldo (server de respaldo — Fase 4)
export interface BackupSession {
  email: string;
  photographer_id: string;
  name: string | null;
  server_url: string;
}

// ============================================================================
// BÚSQUEDA DE FOTOS (spec photo-search)
// ============================================================================

export type StarsFilter = { mode: 'exact' | 'at_least' | 'at_most'; value: number };

export type SearchPeriod =
  | { kind: 'year'; year: number }
  | { kind: 'month'; year: number; month: number };

/** Consulta interpretada: es lo que edita el panel de búsqueda avanzada. */
export interface SearchQuery {
  text: string[];
  tags: string[];
  not_tags: string[];
  stars: StarsFilter | null;
  /** 1 rojo … 5 púrpura */
  colors: number[];
  camera: string | null;
  lens: string | null;
  period: SearchPeriod | null;
  /** `AAAA-MM-DD`, sin incluir el día */
  before: string | null;
  after: string | null;
  project: string | null;
  approved: boolean;
  delivered: boolean;
  pair: boolean;
  include_archived: boolean;
}

export interface SearchHit {
  photo_id: string;
  project_id: string;
  project_name: string;
  project_date: string | null;
  /** Último disco donde se vio el proyecto */
  device_name: string | null;
  file_name: string;
  capture_date: string | null;
  stars: number;
  colors: number[];
  tags: string | null;
  culled: boolean;
  delivered: boolean;
  is_pair: boolean;
  /** El disco del proyecto está conectado: se puede abrir */
  connected: boolean;
  /** Parecido con el texto buscado, si salió por contenido (semantic-search) */
  similarity?: number;
}

export interface SearchResult {
  hits: SearchHit[];
  /** Había más resultados que los mostrados */
  truncated: boolean;
  /** Proyectos con fotos sin analizar entre las buscadas (solo al buscar por contenido) */
  unanalyzed: { project_id: string; count: number }[];
}

/** Búsqueda de fotos en curso en el dashboard: se conserva al abrir una foto y volver. */
export interface SearchSnapshot {
  query: string;
  result: SearchResult | null;
  scrollTop: number;
}

// ============================================================================
// ÁLBUMES (spec photo-albums)
// ============================================================================

export type AlbumItemKind = 'photo' | 'export';
export type AlbumItemStatus = 'final' | 'unedited' | 'missing';
export type AlbumOrder = 'added' | 'captured';

/** De dónde sale la miniatura de un elemento en el índice. */
export type AlbumThumbKey = { kind: 'photo' | 'export'; id: string };

/** Un elemento por agregar a un álbum (o con qué reemplazar otro). */
export interface NewAlbumItem {
  kind: AlbumItemKind;
  project_id: string;
  ref_id: string;
  file_name: string;
  project_name: string;
}

export interface AlbumSummary {
  id: string;
  name: string;
  created_at: string;
  items: number;
  finals: number;
  cover: AlbumThumbKey | null;
}

export interface AlbumEntry {
  item_id: string;
  kind: AlbumItemKind;
  project_id: string;
  ref_id: string;
  status: AlbumItemStatus;
  file_name: string;
  project_name: string;
  device_name: string | null;
  capture_date: string | null;
  stars: number;
  colors: number[];
  culled: boolean;
  is_pair: boolean;
  /** Foto que se abre en el visor (un exportado suelto no tiene). */
  photo_id: string | null;
  thumb: AlbumThumbKey | null;
  added_at: string;
  connected: boolean;
}

export interface AlbumView {
  id: string;
  name: string;
  entries: AlbumEntry[];
  finals: number;
  unedited: number;
  missing: number;
  missing_disks: { device_name: string | null; photos: number }[];
}

export interface ReplaceCandidate {
  item: NewAlbumItem;
  capture_date: string | null;
  thumb: AlbumThumbKey;
}
