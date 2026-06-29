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
