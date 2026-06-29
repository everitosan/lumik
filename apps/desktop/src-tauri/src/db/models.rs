use serde::{Deserialize, Serialize};

/// Photographer (local user)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photographer {
    pub id: String,
    pub email: String,
    pub alias: String,
    pub active: bool,
    pub deleted: bool,
}

/// Photography project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub session_date: Option<String>,
    pub archived: bool,
    pub deleted: bool,
    pub creator_id: String,
    pub cover_photo_path: Option<String>,
}

/// Workflow status
// Representación tipada de workflow_status. Hoy se guarda/lee como String crudo
// (con CHECK en el schema); este enum queda reservado para cuando se cablee.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum WorkflowStatus {
    Imported,
    Edited,
    Delivered,
}

#[allow(dead_code)]
impl WorkflowStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkflowStatus::Imported => "imported",
            WorkflowStatus::Edited => "edited",
            WorkflowStatus::Delivered => "delivered",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "imported" => Some(Self::Imported),
            "edited" => Some(Self::Edited),
            "delivered" => Some(Self::Delivered),
            _ => None,
        }
    }
}

/// Backup status. Reservado: la feature de respaldo está planificada (fuera del
/// MVP); el schema ya reserva las columnas pero aún no se construye este enum.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)]
pub enum BackupStatus {
    Pending,
    Uploaded,
    Failed,
}

/// Photography (photo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Photo {
    pub id: String,
    pub project_id: String,
    pub dng_path: String,
    pub jpg_path: Option<String>,
    pub device_uuid: String,
    pub original_camera: Option<String>,
    pub original_format: Option<String>,
    pub import_date: String,
    pub file_hash: Option<String>,
    pub culled: bool,
    pub workflow_status: String,
    pub backup_status: String,
    pub backup_url: Option<String>,
    pub backup_date: Option<String>,
    pub backup_retries: i32,
    pub deleted: bool,
    pub stars: i32,
    pub color_label: Option<String>,
    pub tags: Option<String>,
    pub capture_date: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub file_size_bytes: Option<i64>,
    pub iso: Option<i32>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub exposure_compensation: Option<f64>,
    pub focal_length: Option<String>,
    pub lens_model: Option<String>,
    pub rotation: i32,
}

/// Photographer metadata for XMP writing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotographerMetadata {
    pub id: String,
    pub photographer_id: String,
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub contact_email: Option<String>,
    pub contact_url: Option<String>,
    pub contact_phone: Option<String>,
    pub usage_terms: Option<String>,
    pub custom_tags: Option<String>,
}

/// Known device from the registry (global DB)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownDevice {
    pub uuid: String,
    pub name: String,
    pub mount_hint: Option<String>,
    pub last_seen: String,
    pub registered_at: String,
}

// ============================================================================
// VIEWS (for dashboard queries)
// ============================================================================

/// Project summary for the dashboard — aggregated per ProjectDatabase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDashboard {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub session_date: Option<String>,
    pub photo_count: i32,
    pub workflow_status: String,
    pub cover_photo_path: Option<String>,
    /// UUID of the device where this project's database lives
    pub device_uuid: String,
}

// ============================================================================
// DTOs for create/update
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CreateProject {
    pub name: String,
    pub description: Option<String>,
    pub session_date: Option<String>,
    pub creator_id: String,
    /// UUID of the device where the project.db will be created
    pub device_uuid: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePhotographerMetadata {
    pub artist: Option<String>,
    pub copyright: Option<String>,
    pub contact_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePhoto {
    pub project_id: String,
    pub dng_path: String,
    pub device_uuid: String,
    pub original_camera: Option<String>,
    pub original_format: Option<String>,
    pub capture_date: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub file_size_bytes: Option<i64>,
    pub iso: Option<i32>,
    pub aperture: Option<String>,
    pub shutter_speed: Option<String>,
    pub exposure_compensation: Option<f64>,
    pub focal_length: Option<String>,
    pub lens_model: Option<String>,
    pub rotation: i32,
}

// ============================================================================
// KEYBINDINGS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybinding {
    pub action: String,
    pub key: String,
    pub description: String,
}

// ============================================================================
// SETTINGS
// ============================================================================

/// Application settings with typed values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub embed_metadata_on_import: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            embed_metadata_on_import: true,
        }
    }
}

/// Per-project UI and workflow settings (single row in project_settings).
/// Add new fields with a Default impl so existing code stays valid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    pub sidebar_open: bool,
    pub show_culled: bool,
    #[serde(default)]
    pub min_stars: Option<i32>,
    #[serde(default)]
    pub selected_tags: Option<String>,
    #[serde(default)]
    pub selected_colors: Option<String>,
    #[serde(default)]
    pub stars_filter_mode: Option<String>,
    #[serde(default)]
    pub view_mode: Option<String>,
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            sidebar_open: true,
            show_culled: false,
            min_stars: None,
            selected_tags: None,
            selected_colors: None,
            stars_filter_mode: None,
            view_mode: None,
        }
    }
}
