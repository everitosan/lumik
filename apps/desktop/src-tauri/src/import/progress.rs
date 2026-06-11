use serde::{Deserialize, Serialize};

/// Processing phase for a single file during import
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportPhase {
    Reading,
    Converting,
    Writing,
    Saving,
    Complete,
    Failed,
}

/// Event payload for import progress updates (emitted via Tauri events)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub session_id: String,
    pub current_index: usize,
    pub total_files: usize,
    pub current_file: String,
    pub phase: ImportPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Information about a file that failed to import
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedFile {
    pub name: String,
    pub path: String,
    pub error: String,
}

/// Final result of an import session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub session_id: String,
    pub total_files: usize,
    pub successful: usize,
    pub failed: usize,
    pub failed_files: Vec<FailedFile>,
    pub videos_copied: usize,
}
