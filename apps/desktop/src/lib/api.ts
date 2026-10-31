import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  EditProjectProgress,
  EditProjectRequest,
  EditProjectResult,
  AppSettings,
  DeliveryMode,
  DeliveryPlan,
  DeliveryChangeOutcome,
  DashboardSort,
  DetectedDevice,
  Keybinding,
  KnownDevice,
  Photo,
  Photographer,
  PhotographerMetadata,
  Project,
  ProjectDashboard,
  ProjectSettings,
  CreateProject,
  UpdatePhotographerMetadata,
  ImportRequest,
  ImportResult,
  ImportBreakdown,
  RetryRegisterRequest,
  BackupSession,
  ProjectAccess,
  SearchQuery,
  ContentModelState,
  AnalysisMode,
  ProjectAnalysis,
  AnalysisQueueState,
  AnalysisEvent,
  SearchResult,
  AlbumOrder,
  AlbumSummary,
  AlbumThumbKey,
  AlbumView,
  NewAlbumItem,
  ReplaceCandidate,
} from './types';

// ============================================================================
// CANDADO DE PROYECTO (spec project-lock)
// ============================================================================

/** Prefijo del error de un proyecto que tiene otra instalación. */
const PROJECT_LOCKED_PREFIX = 'project_locked:';

/** Evento de ventana que avisa que una escritura chocó con el candado de otro. */
export const PROJECT_LOCKED_EVENT = 'lumik:project-locked';

/** Nombre del dispositivo que tiene el proyecto, si `err` es el error de candado. */
export function projectLockHolder(err: unknown): string | null {
  const text = typeof err === 'string' ? err : err instanceof Error ? err.message : '';
  return text.startsWith(PROJECT_LOCKED_PREFIX) ? text.slice(PROJECT_LOCKED_PREFIX.length) : null;
}

/**
 * `invoke` de Tauri que, además, avisa por `PROJECT_LOCKED_EVENT` cuando un
 * comando falla porque otra instalación tomó el proyecto, para que la vista
 * pase a solo lectura sin que cada llamada lo maneje.
 */
async function invoke<T>(cmd: string, args?: Parameters<typeof tauriInvoke>[1]): Promise<T> {
  try {
    return await tauriInvoke<T>(cmd, args);
  } catch (err) {
    const holder = projectLockHolder(err);
    if (holder !== null) {
      window.dispatchEvent(new CustomEvent(PROJECT_LOCKED_EVENT, { detail: holder }));
    }
    throw err;
  }
}

/** Abre la vista de un proyecto: toma su candado o la abre en solo lectura. */
export async function openProjectSession(projectId: string): Promise<ProjectAccess> {
  return invoke('open_project_session', { projectId });
}

/** Cierra la vista de un proyecto y suelta su candado si no queda nada en curso. */
export async function closeProjectSession(projectId: string): Promise<void> {
  return invoke('close_project_session', { projectId });
}

/** Toma el control de un proyecto que tiene otra instalación. */
export async function takeProjectControl(projectId: string): Promise<void> {
  return invoke('take_project_control', { projectId });
}

/** Nombre de esta instalación, el que ven otros dispositivos en el candado. */
export async function getInstallName(): Promise<string> {
  return invoke('get_install_name');
}

export async function setInstallName(name: string): Promise<string> {
  return invoke('set_install_name', { name });
}

// ============================================================================
// PLATFORM
// ============================================================================

export type Platform = 'linux' | 'windows' | 'macos';

export const getPlatform = (): Promise<Platform> =>
  invoke<Platform>('get_platform');

// ============================================================================
// DEVICE API (runtime scan only, no DB storage)
// ============================================================================

export async function scanConnectedDevices(): Promise<DetectedDevice[]> {
  return invoke('scan_connected_devices');
}

/// Safely release an external device so the OS can eject it, without closing the app.
/// Closes all project DBs on the device and asks the OS to unmount it.
export async function ejectDevice(deviceUuid: string): Promise<void> {
  return invoke('eject_device', { deviceUuid });
}

export async function getKnownDevices(): Promise<KnownDevice[]> {
  return invoke('get_known_devices');
}

// ============================================================================
// PROJECT API
// ============================================================================

export async function getProjectsDashboard(): Promise<ProjectDashboard[]> {
  return invoke('get_projects_dashboard');
}

export async function getProject(id: string): Promise<Project | null> {
  return invoke('get_project', { id });
}

export async function createProject(project: CreateProject): Promise<Project> {
  return invoke('create_project', { project });
}

export async function getProjectSettings(projectId: string): Promise<ProjectSettings> {
  return invoke('get_project_settings', { projectId });
}

export async function updateProjectSettings(projectId: string, settings: ProjectSettings): Promise<void> {
  return invoke('update_project_settings', { projectId, settings });
}

export async function getProjectDeliveryMode(projectId: string): Promise<DeliveryMode> {
  return invoke('get_project_delivery_mode', { projectId });
}

export async function planProjectDeliveryChange(projectId: string, mode: DeliveryMode): Promise<DeliveryPlan> {
  return invoke('plan_project_delivery_change', { projectId, mode });
}

/** Cambia la entrega del proyecto; con `applyToApproved`, también en las fotos ya aprobadas. */
export async function setProjectDeliveryMode(
  projectId: string,
  mode: DeliveryMode,
  applyToApproved = false,
): Promise<DeliveryChangeOutcome | null> {
  return invoke('set_project_delivery_mode', { projectId, mode, applyToApproved });
}

export async function getProjectPhotos(projectId: string): Promise<Photo[]> {
  return invoke('get_project_photos', { projectId });
}

/// Photos in the trash (logical delete), for the "Trash" filter.
export async function getProjectTrashedPhotos(projectId: string): Promise<Photo[]> {
  return invoke('get_project_trashed_photos', { projectId });
}

/// Logical delete: moves the photo (and its export) to the project's trash, `_trash/`.
export async function trashPhoto(projectId: string, photoId: string): Promise<void> {
  return invoke('trash_photo', { projectId, photoId });
}

/// Moves a trashed photo back to `_media/`/`_culled/` (and its export to `_exported/`).
export async function restorePhoto(projectId: string, photoId: string): Promise<void> {
  return invoke('restore_photo', { projectId, photoId });
}

export async function getProjectThumbnails(projectId: string): Promise<string[]> {
  return invoke('get_project_thumbnails', { projectId });
}

export async function getThumbnail(
  projectId: string,
  photoId: string
): Promise<string | null> {
  return invoke('get_thumbnail', { projectId, photoId });
}

export interface PhotoPreviewResult {
  url: string;
  rotation: number;
}

/** Vista previa de una foto; de un par RAW+JPEG, su JPEG salvo que se pida el RAW. */
export async function getPhotoPreview(
  photoId: string,
  projectId: string,
  raw = false,
): Promise<PhotoPreviewResult> {
  return invoke('get_photo_preview', { photoId, projectId, raw });
}

export async function savePhotoRotation(
  photoId: string,
  projectId: string,
  rotation: number,
): Promise<void> {
  return invoke('save_photo_rotation', { photoId, projectId, rotation });
}

export async function regenerateProjectThumbnails(projectId: string): Promise<number> {
  return invoke('regenerate_project_thumbnails', { projectId });
}

export async function savePhotoRating(
  photoId: string,
  projectId: string,
  stars: number,
  colorLabel: string | null,
  tags: string | null,
): Promise<void> {
  return invoke('save_photo_rating', { photoId, projectId, stars, colorLabel, tags });
}

export async function savePhotoCulled(
  photoId: string,
  projectId: string,
  culled: boolean,
): Promise<void> {
  return invoke('save_photo_culled', { photoId, projectId, culled });
}

export async function getProjectCoverThumbnail(
  projectId: string,
): Promise<string | null> {
  return invoke('get_project_cover_thumbnail', { projectId });
}

export async function setProjectCoverPhoto(
  projectId: string,
  photoId: string | null,
): Promise<void> {
  return invoke('set_project_cover_photo', { projectId, photoId });
}

/** Sube la portada del proyecto al catálogo (para el dashboard web). Fire-and-forget. */
export async function syncProjectCover(projectId: string): Promise<void> {
  return invoke('sync_project_cover', { projectId });
}

/**
 * Tras un respaldo exitoso, fija el estado del proyecto (respaldado/editando) según
 * _exported vs _culled y guarda los proveedores usados. Devuelve el estado nuevo.
 */
export async function finalizeBackup(
  projectId: string,
  providers?: string
): Promise<string> {
  return invoke('finalize_backup', { projectId, providers: providers ?? null });
}

/** Abre una URL externa (web del proveedor) en el navegador del SO. */
export async function openExternalUrl(url: string): Promise<void> {
  return invoke('open_external_url', { url });
}

/**
 * Enlace expirable a la carpeta del proyecto en un proveedor, o `null` si el
 * proveedor no soporta expiración (el caller abre la web del proveedor).
 */
export async function getBackupFolderLink(
  projectId: string,
  provider: string
): Promise<string | null> {
  return invoke('get_backup_folder_link', { projectId, provider });
}

export async function setProjectArchived(id: string, archived: boolean): Promise<void> {
  return invoke('set_project_archived', { id, archived });
}

export async function getDashboardShowArchived(): Promise<boolean> {
  return invoke('get_dashboard_show_archived');
}

export async function setDashboardShowArchived(show: boolean): Promise<boolean> {
  return invoke('set_dashboard_show_archived', { show });
}

export type SidebarSection = 'backups' | 'storage';

export interface SidebarSections {
  backups_collapsed: boolean;
  storage_collapsed: boolean;
}

export async function getSidebarSections(): Promise<SidebarSections> {
  return invoke('get_sidebar_sections');
}

export async function setSidebarSectionCollapsed(
  section: SidebarSection,
  collapsed: boolean
): Promise<SidebarSections> {
  return invoke('set_sidebar_section_collapsed', { section, collapsed });
}

export async function deleteProject(id: string): Promise<void> {
  return invoke('delete_project', { id });
}

/// Edita nombre, fecha y descripción; mueve la carpeta si hace falta y, si se
/// pide, renombra las fotos y reescribe su descripción. `onProgress` recibe el
/// avance de la reescritura de metadatos.
export async function editProject(
  request: EditProjectRequest,
  onProgress?: (progress: EditProjectProgress) => void,
): Promise<EditProjectResult> {
  const unlisten = onProgress
    ? await listen<EditProjectProgress>('edit-project-progress', (e) => {
        if (e.payload.project_id === request.id) onProgress(e.payload);
      })
    : undefined;
  try {
    return await invoke('edit_project', { request });
  } finally {
    unlisten?.();
  }
}

/// Open the project's folder in the OS file manager (desktop only).
export async function openProjectFolder(id: string): Promise<void> {
  return invoke('open_project_folder', { id });
}

/// Open the OS file manager with the photo's original file selected (desktop only).
export async function revealPhoto(projectId: string, photoId: string): Promise<void> {
  return invoke('reveal_photo', { projectId, photoId });
}

// ============================================================================
// PHOTOGRAPHER API
// ============================================================================

export async function getActivePhotographer(): Promise<Photographer | null> {
  return invoke('get_active_photographer');
}

export async function ensureDefaultPhotographer(
  email: string,
  alias: string
): Promise<Photographer> {
  return invoke('ensure_default_photographer', { email, alias });
}

// ============================================================================
// PHOTOGRAPHER METADATA API
// ============================================================================

export async function getPhotographerMetadata(
  photographerId: string
): Promise<PhotographerMetadata | null> {
  return invoke('get_photographer_metadata', { photographerId });
}

export async function updatePhotographerMetadata(
  photographerId: string,
  metadata: UpdatePhotographerMetadata
): Promise<PhotographerMetadata> {
  return invoke('update_photographer_metadata', { photographerId, metadata });
}

// ============================================================================
// SETTINGS API
// ============================================================================

// ============================================================================
// KEYBINDING API
// ============================================================================

export async function getKeybindings(): Promise<Keybinding[]> {
  return invoke('get_keybindings');
}

export async function updateKeybinding(action: string, key: string): Promise<void> {
  return invoke('update_keybinding', { action, key });
}

// ============================================================================
// SETTINGS API
// ============================================================================

export async function getAppSettings(): Promise<AppSettings> {
  return invoke('get_app_settings');
}

export async function updateAppSettings(
  settings: AppSettings
): Promise<AppSettings> {
  return invoke('update_app_settings', { settings });
}

export async function getDashboardSort(): Promise<DashboardSort> {
  return invoke('get_dashboard_sort');
}

export async function updateDashboardSort(sort: DashboardSort): Promise<DashboardSort> {
  return invoke('update_dashboard_sort', { sort });
}

// ============================================================================
// IMPORT API
// ============================================================================

/** Desglose por tomas de los archivos elegidos, antes de importar. */
export async function analyzeImportSelection(projectId: string, sourceFiles: string[]): Promise<ImportBreakdown> {
  return invoke('analyze_import_selection', { projectId, sourceFiles });
}

export async function startImport(request: ImportRequest): Promise<ImportResult> {
  return invoke('start_import', { request });
}

export async function retryImportRegister(request: RetryRegisterRequest): Promise<ImportResult> {
  return invoke('retry_import_register', { request });
}

// ============================================================================
// BACKUP ACCOUNT API (server de respaldo — Fase 4)
// ============================================================================

export async function getBackupServerUrl(): Promise<string> {
  return invoke('get_backup_server_url');
}

export async function setBackupServerUrl(url: string): Promise<void> {
  return invoke('set_backup_server_url', { url });
}

export async function backupLogin(
  email: string,
  password: string
): Promise<BackupSession> {
  return invoke('backup_login', { email, password });
}

export async function backupRegister(
  email: string,
  password: string,
  name?: string
): Promise<BackupSession> {
  return invoke('backup_register', { email, password, name: name ?? null });
}

export async function backupLogout(): Promise<void> {
  return invoke('backup_logout');
}

export async function getBackupSession(): Promise<BackupSession | null> {
  return invoke('get_backup_session');
}

export type SyncOutcome =
  | 'skipped_no_session'
  | 'offline'
  | 'already_exists'
  | 'published';

/**
 * Asegura que el proyecto exista en el catálogo del server (Fase 5). Se llama al
 * abrir un proyecto; degrada a solo-local sin sesión/red. Fire-and-forget.
 */
export async function ensureProjectSynced(
  projectId: string
): Promise<SyncOutcome> {
  return invoke('ensure_project_synced', { projectId });
}

export interface SyncExportedResult {
  matched: number;
  unmatched: string[];
}

/**
 * Escanea _exported/ y enlaza cada archivo con su foto (jpg_path + edited), luego
 * publica los cambios al catálogo (Fase 6). Fire-and-forget al abrir el proyecto.
 */
export async function syncExported(
  projectId: string
): Promise<SyncExportedResult> {
  return invoke('sync_exported', { projectId });
}

export interface ExportedBackupItem {
  id: string;
  filename: string;
  in_progress: boolean;
}

export interface ProviderInfo {
  provider: string;
  name: string;
}

/** Archivos de _exported/ pendientes de respaldo (los ya subidos se excluyen). */
export async function listExportedBackup(
  projectId: string
): Promise<ExportedBackupItem[]> {
  return invoke('list_exported_backup', { projectId });
}

/** Proveedores de nube conectados (para los toggles del modal). */
export async function listBackupProviders(): Promise<ProviderInfo[]> {
  return invoke('list_backup_providers');
}

/** Crea el directorio remoto _exported/ una vez, antes de subir en paralelo. */
export async function prepareBackup(
  projectId: string,
  targets?: string
): Promise<void> {
  return invoke('prepare_backup', { projectId, targets: targets ?? null });
}

/**
 * Renombra la carpeta del backup en la nube para que coincida con el nombre/
 * fecha actuales del proyecto (tras un rename). Best-effort en el server: si no
 * hay sesión o nada se ha respaldado, no hace nada.
 */
export async function relocateBackup(projectId: string): Promise<void> {
  return invoke('relocate_backup', { projectId });
}

/**
 * Respalda un archivo de _exported/ (uno por llamada, para feedback por-foto).
 * `targets`: proveedores separados por coma (o undefined = todos).
 * `quality`: 'original' | 'high' | 'medium' | 'low'.
 */
export async function backupExportedFile(
  projectId: string,
  id: string,
  filename: string,
  targets?: string,
  quality?: string
): Promise<string> {
  return invoke('backup_exported_file', {
    projectId,
    id,
    filename,
    targets: targets ?? null,
    quality: quality ?? null,
  });
}

// ============================================================================
// BACKUP QUEUE (cola offline-first — Fase 9)
// ============================================================================

export interface PendingBackup {
  project_id: string;
  targets: string | null;
  quality: string | null;
}

/** Encola/actualiza la intención de respaldo de un proyecto. */
export async function setPendingBackup(
  projectId: string,
  targets?: string,
  quality?: string
): Promise<void> {
  return invoke('set_pending_backup', {
    projectId,
    targets: targets ?? null,
    quality: quality ?? null,
  });
}

/** Quita un proyecto de la cola (respaldo completado). */
export async function clearPendingBackup(projectId: string): Promise<void> {
  return invoke('clear_pending_backup', { projectId });
}

/** Lista los respaldos pendientes (para reanudar al arrancar). */
export async function getPendingBackups(): Promise<PendingBackup[]> {
  return invoke('get_pending_backups');
}

// ============================================================================
// BÚSQUEDA DE FOTOS (spec photo-search)
// ============================================================================

/** Busca en el índice de todos los proyectos que ha visto este equipo. */
export async function searchPhotos(query: string): Promise<SearchResult> {
  return invoke('search_photos', { query });
}

/** Miniatura del índice de una foto (data URL), aunque su disco no esté conectado. */
export async function getSearchThumbnail(photoId: string): Promise<string | null> {
  return invoke('get_search_thumbnail', { photoId });
}

/** Cuántas fotos hay en el índice. */
export async function getSearchIndexCount(): Promise<number> {
  return invoke('get_search_index_count');
}

/** Interpreta lo escrito en el cuadro (para precargar el panel avanzado). */
export async function parseSearchQuery(query: string): Promise<SearchQuery> {
  return invoke('parse_search_query', { query });
}

/** Escribe lo elegido en el panel como operadores, en el idioma de la app. */
export async function formatSearchQuery(query: SearchQuery, lang: 'es' | 'en'): Promise<string> {
  return invoke('format_search_query', { query, lang });
}

// ── Modelo de búsqueda por contenido (semantic-search) ──────────────────────

export async function getContentModelState(): Promise<ContentModelState> {
  return invoke('get_content_model_state');
}

/** Empieza la descarga; el progreso y el resultado llegan con `onContentModelState`. */
export async function downloadContentModel(): Promise<void> {
  return invoke('download_content_model');
}

export async function cancelContentModelDownload(): Promise<void> {
  return invoke('cancel_content_model_download');
}

export async function deleteContentModel(): Promise<void> {
  return invoke('delete_content_model');
}

export function onContentModelState(handler: (state: ContentModelState) => void): Promise<() => void> {
  return listen<ContentModelState>('content-model-state', (e) => handler(e.payload));
}

export async function getProjectAnalysis(projectId: string): Promise<ProjectAnalysis> {
  return invoke('get_project_analysis', { projectId });
}

export async function setProjectAnalysisMode(projectId: string, mode: AnalysisMode): Promise<void> {
  return invoke('set_project_analysis_mode', { projectId, mode });
}

/** "Analizar fotos": encola el proyecto. Falla con `no_model` si el modelo no está instalado. */
export async function analyzeProject(projectId: string): Promise<void> {
  return invoke('analyze_project', { projectId });
}

export async function stopProjectAnalysis(projectId: string): Promise<void> {
  return invoke('stop_project_analysis', { projectId });
}

export async function getAnalysisQueue(): Promise<AnalysisQueueState> {
  return invoke('get_analysis_queue');
}

export function onAnalysisProgress(handler: (event: AnalysisEvent) => void): Promise<() => void> {
  return listen<AnalysisEvent>('analysis-progress', (e) => handler(e.payload));
}

// ============================================================================
// ÁLBUMES (spec photo-albums)
// ============================================================================

export async function listAlbums(): Promise<AlbumSummary[]> {
  return invoke('list_albums');
}

export async function createAlbum(name: string): Promise<AlbumSummary> {
  return invoke('create_album', { name });
}

export async function renameAlbum(albumId: string, name: string): Promise<void> {
  return invoke('rename_album', { albumId, name });
}

export async function deleteAlbum(albumId: string): Promise<void> {
  return invoke('delete_album', { albumId });
}

/** Agrega fotos o exportados; devuelve cuántos entraron (los repetidos no). */
export async function addToAlbum(albumId: string, items: NewAlbumItem[]): Promise<number> {
  return invoke('add_to_album', { albumId, items });
}

/** Agrega todas las fotos que encuentra una búsqueda. */
export async function addSearchToAlbum(albumId: string, query: string): Promise<number> {
  return invoke('add_search_to_album', { albumId, query });
}

export async function removeAlbumItem(itemId: string): Promise<void> {
  return invoke('remove_album_item', { itemId });
}

export async function replaceAlbumItem(itemId: string, withItem: NewAlbumItem): Promise<void> {
  return invoke('replace_album_item', { itemId, with: withItem });
}

export async function getAlbum(albumId: string, order: AlbumOrder): Promise<AlbumView | null> {
  return invoke('get_album', { albumId, order });
}

export async function getReplaceCandidates(projectId: string): Promise<ReplaceCandidate[]> {
  return invoke('get_replace_candidates', { projectId });
}

/** Miniatura de un elemento (data URL), aunque su disco no esté conectado. */
export async function getAlbumThumbnail(key: AlbumThumbKey): Promise<string | null> {
  return invoke('get_album_thumbnail', { key });
}

/** Muestra en la carpeta el archivo de un elemento. Sin su disco, falla con `disk_missing:<disco>`. */
export async function revealAlbumItem(itemId: string): Promise<void> {
  return invoke('reveal_album_item', { itemId });
}

/** Nombre del disco que falta, si `err` es el error de `revealAlbumItem` sin disco. */
export function missingDiskOf(err: unknown): string | null {
  const text = typeof err === 'string' ? err : err instanceof Error ? err.message : '';
  return text.startsWith('disk_missing:') ? text.slice('disk_missing:'.length) : null;
}
