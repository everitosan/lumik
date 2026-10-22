//! Implementación de `ProgressReporter` sobre eventos Tauri. Se construye por
//! sesión de import a partir del `AppHandle` del comando.

use crate::application::ports::ProgressReporter;
use crate::import::{ImportLogEntry, ImportProgress};
use log::warn;
use tauri::{AppHandle, Emitter};

/// Reporter que emite `import-log` / `import-progress` al frontend.
pub struct TauriProgressReporter {
    pub app: AppHandle,
}

impl ProgressReporter for TauriProgressReporter {
    fn log(&self, entry: ImportLogEntry) {
        if let Err(e) = self.app.emit("import-log", &entry) {
            warn!("Failed to emit log event: {}", e);
        }
    }

    fn progress(&self, progress: ImportProgress) {
        if let Err(e) = self.app.emit("import-progress", &progress) {
            warn!("Failed to emit progress event: {}", e);
        }
    }
}
