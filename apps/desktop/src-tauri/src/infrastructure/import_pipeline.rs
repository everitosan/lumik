//! Implementación real de `ImportPipeline` sobre el módulo `crate::import`
//! (workspace temporal + exiftool + movimiento a disco). Emite el progreso de
//! las fases copiar/metadata/mover vía el puerto `ProgressReporter`.

use crate::application::ports::{ImportPipeline, PipelinePhotos, ProgressReporter};
use crate::import::{
    is_video_file as _is_video_file, pipeline_copy_videos, pipeline_metadata,
    pipeline_move_to_dest, pipeline_passthrough, ImportPhase, ImportProgress, PipelineWorkspace,
};
use log::info;
use std::path::{Path, PathBuf};

pub struct StdImportPipeline;

fn phase(session_id: &str, index: usize, label: &str, phase: ImportPhase) -> ImportProgress {
    ImportProgress {
        session_id: session_id.to_string(),
        current_index: index,
        total_files: 3,
        current_file: label.to_string(),
        phase,
        error: None,
    }
}

impl ImportPipeline for StdImportPipeline {
    fn process_photos(
        &self,
        req: PipelinePhotos,
        reporter: &dyn ProgressReporter,
    ) -> Result<Vec<PathBuf>, String> {
        // === FASE 1: copiar al workspace ===
        reporter.progress(phase(req.session_id, 0, "Copiando archivos", ImportPhase::Reading));

        let workspace = PipelineWorkspace::create(req.project_name)
            .map_err(|e| format!("Failed to create workspace: {}", e))?;
        reporter.log(
            req.session_id,
            &format!("Workspace creado: {}", workspace.temp_dir.display()),
        );

        let copied = pipeline_passthrough(req.photos, &workspace)
            .map_err(|e| format!("Failed to copy files: {}", e))?;
        info!("Copied {} files", copied);
        reporter.log(req.session_id, &format!("{} archivos copiados al workspace", copied));

        // === FASE 2: metadata (embebe XMP + renombra) ===
        reporter.progress(phase(req.session_id, 1, "Agregando metadatos", ImportPhase::Writing));
        reporter.log(req.session_id, "Procesando metadatos XMP y nombres de archivo...");
        pipeline_metadata(
            &workspace,
            req.project_name,
            req.metadata,
            req.image_description,
            req.rename,
        )
        .map_err(|e| format!("Metadata failed: {}", e))?;
        reporter.log(req.session_id, "Metadatos aplicados");

        // === mover a destino ===
        reporter.log(req.session_id, "Moviendo archivos al disco de destino...");
        let dng_files = pipeline_move_to_dest(&workspace, req.dest_folder)
            .map_err(|e| format!("Move failed: {}", e))?;
        reporter.log(
            req.session_id,
            &format!("{} archivos movidos a _media/", dng_files.len()),
        );

        workspace.cleanup();
        Ok(dng_files)
    }

    fn copy_videos(&self, videos: &[PathBuf], dest: &Path) -> Result<usize, String> {
        info!("Copying {} video files to _video/", videos.len());
        pipeline_copy_videos(videos, dest).map_err(|e| format!("Failed to copy videos: {}", e))
    }
}

// Reexport para que `commands.rs` particione sin importar `crate::import` directo.
pub fn is_video_file(path: &Path) -> bool {
    _is_video_file(path)
}
