use serde::{Deserialize, Serialize};

/// Fase del import que se está ejecutando.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportPhase {
    /// Copia de videos al staging.
    Videos,
    /// Copia de fotos al staging, sin nada que escribir.
    Copy,
    /// Escritorio: copia al staging firmando y renombrando en la misma pasada.
    CopyMetadata,
    /// Android: renombrado y sidecar XMP sobre el staging.
    Metadata,
    /// Paso del staging a `_media/`.
    Move,
    /// EXIF, base de datos y miniaturas.
    Register,
    Complete,
}

/// Evento de progreso del import (evento Tauri `import-progress`). `file_index`
/// es el número (desde 1) del archivo en curso dentro de `file_total`; los bytes
/// solo avanzan en las fases que copian desde el origen (`videos`, `copy` y
/// `copy_metadata`). Con varios archivos en paralelo, el "archivo en curso" es
/// el último que empezó y `bytes_done` incluye lo escrito de todos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProgress {
    pub session_id: String,
    pub phase: ImportPhase,
    pub file_name: String,
    pub file_index: usize,
    pub file_total: usize,
    pub file_bytes: u64,
    pub file_bytes_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

/// Motivo de un fallo, traducido en el frontend (`import.errors.codes.<code>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportErrorCode {
    SourceUnreadable,
    NoSpace,
    WriteFailed,
    MetadataFailed,
    RegisterFailed,
    Unknown,
}

/// Lado de una operación de E/S, para elegir el código cuando no es falta de espacio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSide {
    Read,
    Write,
}

/// Código de error para un fallo de E/S: la falta de espacio se reconoce en
/// cualquier lado; el resto depende de si falló leer el origen o escribir.
pub fn error_code_for(err: &std::io::Error, side: IoSide) -> ImportErrorCode {
    if is_no_space(err) {
        return ImportErrorCode::NoSpace;
    }
    match side {
        IoSide::Read => ImportErrorCode::SourceUnreadable,
        IoSide::Write => ImportErrorCode::WriteFailed,
    }
}

fn is_no_space(err: &std::io::Error) -> bool {
    if err.kind() == std::io::ErrorKind::StorageFull {
        return true;
    }
    #[cfg(windows)]
    const NO_SPACE_CODES: &[i32] = &[112, 39]; // ERROR_DISK_FULL, ERROR_HANDLE_DISK_FULL
    #[cfg(not(windows))]
    const NO_SPACE_CODES: &[i32] = &[28]; // ENOSPC
    err.raw_os_error().is_some_and(|c| NO_SPACE_CODES.contains(&c))
}

/// Archivo que no se pudo importar. `error` es el detalle técnico; la UI muestra
/// el motivo traducido a partir de `code`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedFile {
    pub name: String,
    pub path: String,
    pub code: ImportErrorCode,
    pub error: String,
}

impl FailedFile {
    pub fn new(path: &std::path::Path, code: ImportErrorCode, error: impl Into<String>) -> Self {
        Self {
            name: path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            path: path.to_string_lossy().to_string(),
            code,
            error: error.into(),
        }
    }
}

/// Por qué un archivo seleccionado se ignoró sin intentar importarlo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    UnsupportedFormat,
    NotAFile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedFile {
    pub name: String,
    pub path: String,
    pub reason: SkipReason,
}

impl SkippedFile {
    pub fn new(path: &std::path::Path, reason: SkipReason) -> Self {
        Self {
            name: path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            path: path.to_string_lossy().to_string(),
            reason,
        }
    }
}

/// Error que terminó el import antes de tiempo (no ligado a un archivo).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FatalError {
    pub code: ImportErrorCode,
    pub detail: String,
}

/// Tipo de entrada del registro de actividad (`import.log.<kind>` en el frontend).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportLogKind {
    VideoCopied,
    FileCopied,
    FileFailed,
    FileSkipped,
    PhotosRegistered,
    Thumbnails,
    Thumbnail,
    FolderMoved,
    Done,
}

/// Entrada del registro de actividad (evento Tauri `import-log`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportLogEntry {
    pub session_id: String,
    pub kind: ImportLogKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<ImportErrorCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ImportLogEntry {
    pub fn new(session_id: &str, kind: ImportLogKind) -> Self {
        Self {
            session_id: session_id.to_string(),
            kind,
            file_name: None,
            code: None,
            count: None,
            detail: None,
        }
    }

    pub fn file(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    pub fn code(mut self, code: ImportErrorCode) -> Self {
        self.code = Some(code);
        self
    }

    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Resultado final de una sesión de import. Se devuelve aunque el import
/// termine por un error general (`fatal`), con lo que sí quedó importado.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub session_id: String,
    pub total_files: usize,
    pub successful: usize,
    pub failed: usize,
    pub failed_files: Vec<FailedFile>,
    pub skipped_files: Vec<SkippedFile>,
    pub videos_copied: usize,
    pub fatal: Option<FatalError>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error, ErrorKind};

    #[test]
    fn no_space_wins_over_side() {
        let full = Error::from(ErrorKind::StorageFull);
        assert_eq!(error_code_for(&full, IoSide::Read), ImportErrorCode::NoSpace);
        assert_eq!(error_code_for(&full, IoSide::Write), ImportErrorCode::NoSpace);
        #[cfg(not(windows))]
        assert_eq!(
            error_code_for(&Error::from_raw_os_error(28), IoSide::Write),
            ImportErrorCode::NoSpace
        );
        #[cfg(windows)]
        assert_eq!(
            error_code_for(&Error::from_raw_os_error(112), IoSide::Write),
            ImportErrorCode::NoSpace
        );
    }

    #[test]
    fn other_errors_depend_on_side() {
        let denied = Error::from(ErrorKind::PermissionDenied);
        assert_eq!(error_code_for(&denied, IoSide::Read), ImportErrorCode::SourceUnreadable);
        assert_eq!(error_code_for(&denied, IoSide::Write), ImportErrorCode::WriteFailed);
        let missing = Error::from(ErrorKind::NotFound);
        assert_eq!(error_code_for(&missing, IoSide::Read), ImportErrorCode::SourceUnreadable);
    }

    #[test]
    fn serializes_snake_case() {
        let entry = ImportLogEntry::new("s1", ImportLogKind::FileFailed)
            .file("A.CR3")
            .code(ImportErrorCode::NoSpace);
        let json = serde_json::to_value(&entry).unwrap();
        assert_eq!(json["kind"], "file_failed");
        assert_eq!(json["code"], "no_space");
        assert_eq!(json["file_name"], "A.CR3");
        assert!(json.get("count").is_none(), "los opcionales vacíos no se envían");

        let skipped = SkippedFile::new(std::path::Path::new("/src/D.png"), SkipReason::UnsupportedFormat);
        let json = serde_json::to_value(&skipped).unwrap();
        assert_eq!(json["reason"], "unsupported_format");
        assert_eq!(json["name"], "D.png");

        let progress = ImportProgress {
            session_id: "s1".into(),
            phase: ImportPhase::Metadata,
            file_name: String::new(),
            file_index: 0,
            file_total: 0,
            file_bytes: 0,
            file_bytes_total: 0,
            bytes_done: 0,
            bytes_total: 0,
        };
        assert_eq!(serde_json::to_value(&progress).unwrap()["phase"], "metadata");
    }
}
