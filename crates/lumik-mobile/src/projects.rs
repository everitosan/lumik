//! El menú de proyectos del dashboard (editar, archivar) sobre la copia de
//! trabajo de `project.db` en la sandbox, con las reglas del desktop
//! (`edit_project`, `set_project_archived`). Mover la carpeta y devolver la
//! base al disco lo hace la app por SAF.

use lumik_core::application::use_cases::rename_project_files::{PhotoFile, RenameProjectFiles};
use lumik_core::db::ProjectDatabase;
use lumik_core::domain::project::{effective_session_date, ProjectFolder};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::disk::{open, DiskFiles, NoFinderTags, SafFileStore, DISK_ROOT};
use crate::mirror::MirrorError;

/// Lo que muestra la tarjeta del dashboard, como `ProjectDashboard` del desktop.
#[derive(uniffi::Record)]
pub struct ProjectCard {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub session_date: Option<String>,
    pub photo_count: i32,
    /// `imported`, `edited` o `delivered`.
    pub workflow_status: String,
    pub archived: bool,
    /// Portada (`.thumbs/{id}.jpg`, relativa al proyecto): la elegida o la primera foto por fecha.
    pub cover: Option<String>,
}

/// Tarjeta del proyecto de una copia de su `project.db` (la copia puede
/// migrarse; el original del disco no se toca). `None` si está borrado.
#[uniffi::export]
pub fn project_card(db_path: String) -> Result<Option<ProjectCard>, MirrorError> {
    let path = PathBuf::from(&db_path);
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let db = ProjectDatabase::open(path, "", dir).map_err(|e| MirrorError::Db(e.to_string()))?;
    let Some(entry) = db.get_project_dashboard_entry().map_err(|e| MirrorError::Db(e.to_string()))? else {
        return Ok(None);
    };
    let description = db.get_project().map_err(|e| MirrorError::Db(e.to_string()))?.and_then(|p| p.description);
    let cover = db.effective_cover_path().map_err(|e| MirrorError::Db(e.to_string()))?;
    Ok(Some(ProjectCard {
        id: entry.id,
        name: entry.name,
        description,
        created_at: entry.created_at,
        session_date: entry.session_date,
        photo_count: entry.photo_count,
        workflow_status: entry.workflow_status,
        archived: entry.archived,
        cover,
    }))
}

#[uniffi::export]
pub fn set_project_archived(mirror_dir: String, archived: bool) -> Result<(), MirrorError> {
    open(&mirror_dir)?.set_archived(archived).map_err(|e| MirrorError::Db(e.to_string()))
}

#[derive(uniffi::Record)]
pub struct EditOutcome {
    /// Carpeta que le toca al proyecto bajo `lumik/`: `{año}/{mes}/{día}_{nombre}`.
    pub folder: String,
    pub renamed_files: u32,
}

/// Editar proyecto, como `edit_project` del desktop: valida, guarda nombre,
/// fecha (con la misma regla de fecha efectiva) y descripción en la copia de
/// trabajo y, si se pide, renombra las fotos con el `RenameProjectFiles` del
/// núcleo sobre el disco (todo o nada). Devuelve la carpeta que le toca.
#[uniffi::export]
pub fn edit_project_details(
    mirror_dir: String,
    name: String,
    session_date: String,
    description: Option<String>,
    rename_files: bool,
    disk: Arc<dyn DiskFiles>,
) -> Result<EditOutcome, MirrorError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(MirrorError::Db("El nombre del proyecto es requerido".into()));
    }
    let submitted = session_date.trim().to_string();
    let (year, month, day) = ProjectFolder::date_parts(&submitted)
        .ok_or_else(|| MirrorError::Db(format!("Fecha de sesión no válida: '{submitted}'")))?;
    let description = description.as_deref().map(str::trim).filter(|d| !d.is_empty()).map(str::to_string);

    let db = open(&mirror_dir)?;
    let db_err = |e: lumik_core::db::DbError| MirrorError::Db(e.to_string());
    let current = db.get_project().map_err(db_err)?.ok_or_else(|| MirrorError::Db("No existe el proyecto".into()))?;
    let date = effective_session_date(current.session_date.as_deref(), &current.created_at, &submitted);
    db.update_project_details(&name, date.as_deref(), description.as_deref()).map_err(db_err)?;

    let mut renamed_files = 0;
    if rename_files && current.name.trim() != name {
        let photos: Vec<PhotoFile> = db
            .get_project_photos()
            .map_err(db_err)?
            .into_iter()
            .map(|p| PhotoFile { id: p.id, dng_path: p.dng_path })
            .collect();
        let files = SafFileStore(disk);
        renamed_files = RenameProjectFiles {
            photos: &db,
            files: &files,
            finder_tags: &NoFinderTags,
            project_dir: Path::new(DISK_ROOT),
        }
        .execute(&photos, &current.name, &name)
        .map_err(MirrorError::Db)? as u32;
    }

    Ok(EditOutcome { folder: format!("{year}/{month}/{}", ProjectFolder::folder_name(&day, &name)), renamed_files })
}

/// Mueve una carpeta de la sandbox (el caché de un proyecto que cambió de carpeta).
#[uniffi::export]
pub fn move_dir(from: String, to: String) -> Result<(), MirrorError> {
    if !Path::new(&from).exists() {
        return Ok(());
    }
    if let Some(parent) = Path::new(&to).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let _ = std::fs::remove_dir_all(&to);
    std::fs::rename(&from, &to)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk::{DiskFile, DiskFingerprint};
    use crate::disk::DiskError;

    struct NoDisk;
    impl DiskFiles for NoDisk {
        fn create_dir_all(&self, _rel: String) -> Result<(), DiskError> { Ok(()) }
        fn rename(&self, _from: String, _to: String) -> Result<(), DiskError> { Ok(()) }
        fn exists(&self, _rel: String) -> bool { false }
        fn copy(&self, _from: String, _to: String) -> Result<(), DiskError> { Ok(()) }
        fn remove_file(&self, _rel: String) -> Result<(), DiskError> { Ok(()) }
        fn fingerprint(&self, _rel: String) -> Result<DiskFingerprint, DiskError> {
            Err(DiskError::Failed { message: "sin huella".into() })
        }
        fn list_files(&self, _rel: String) -> Result<Vec<DiskFile>, DiskError> { Ok(vec![]) }
    }

    /// Con un proyecto real: `LUMIK_PROJECT=/ruta cargo test -- --ignored`, sobre una copia.
    #[test]
    #[ignore]
    fn card_archive_and_edit_on_a_copy() {
        let real = PathBuf::from(std::env::var("LUMIK_PROJECT").expect("LUMIK_PROJECT"));
        let copy = tempfile::tempdir().unwrap();
        std::fs::copy(real.join("project.db"), copy.path().join("project.db")).unwrap();
        let mirror = copy.path().display().to_string();
        let db = copy.path().join("project.db").display().to_string();

        let card = project_card(db.clone()).unwrap().unwrap();
        assert!(card.photo_count > 0);

        set_project_archived(mirror.clone(), !card.archived).unwrap();
        assert_eq!(project_card(db.clone()).unwrap().unwrap().archived, !card.archived);

        let out = edit_project_details(mirror.clone(), " Otro nombre ".into(), "2025-03-04".into(), Some("  ".into()), false, Arc::new(NoDisk)).unwrap();
        assert_eq!(out.folder, "2025/03/04_Otro nombre");
        let edited = project_card(db).unwrap().unwrap();
        assert_eq!(edited.name, "Otro nombre");
        assert_eq!(edited.session_date.as_deref(), Some("2025-03-04"));
        assert_eq!(edited.description, None);
        assert!(edit_project_details(mirror, "".into(), "2025-03-04".into(), None, false, Arc::new(NoDisk)).is_err());
    }
}
