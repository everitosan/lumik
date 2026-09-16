import { invoke } from '@tauri-apps/api/core';
import type {
  AppSettings,
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
  BackupSession,
} from './types';

// ============================================================================
// PLATFORM
// ============================================================================

export type Platform = 'linux' | 'windows' | 'macos' | 'android' | 'ios';

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

export async function getProjectPhotos(projectId: string): Promise<Photo[]> {
  return invoke('get_project_photos', { projectId });
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

export async function getPhotoPreview(
  photoId: string,
  projectId: string,
): Promise<PhotoPreviewResult> {
  return invoke('get_photo_preview', { photoId, projectId });
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

export async function archiveProject(id: string): Promise<void> {
  return invoke('archive_project', { id });
}

export async function deleteProject(id: string): Promise<void> {
  return invoke('delete_project', { id });
}

/// Rename a project: moves its folder on disk and updates the stored name + photo paths.
export async function renameProject(id: string, newName: string): Promise<Project> {
  return invoke('rename_project', { id, newName });
}

/// Open the project's folder in the OS file manager (desktop only).
export async function openProjectFolder(id: string): Promise<void> {
  return invoke('open_project_folder', { id });
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

// ============================================================================
// IMPORT API
// ============================================================================

export async function startImport(request: ImportRequest): Promise<ImportResult> {
  return invoke('start_import', { request });
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
