//! Papelera en la tablet (spec `trash-bin`): las mismas reglas de rutas del
//! desktop (`domain::paths`, `application::project_trash`) y los casos de uso de
//! fotos (`RestorePhoto` con otro nombre, `PurgePhoto`) sobre la copia de trabajo
//! y el disco por SAF. Los movimientos de carpetas de proyecto los hace la app con
//! SAF; aquí solo se calculan las rutas.

use lumik_core::application::project_trash::restore_rel;
use lumik_core::application::use_cases::trash_photo::{PurgePhoto, RestorePhoto};
use lumik_core::domain::paths::{parse_trashed_at, project_trash_destination};
use std::path::Path;
use std::sync::Arc;

use crate::disk::{open, DiskFiles, NoFinderTags, SafFileStore, DISK_ROOT};
use crate::mirror::MirrorError;

/// `{ruta}` (relativa a `lumik/`) → `_trash/{fecha}/{ruta}`. `now` es la hora
/// local del dispositivo, `AAAA-MM-DD HH:MM:SS` (la da la app).
#[uniffi::export]
pub fn project_trash_rel(rel_under_lumik: String, now: String) -> Result<String, MirrorError> {
    let now = chrono::NaiveDateTime::parse_from_str(&now, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| MirrorError::Io(format!("Fecha inválida {now}: {e}")))?;
    project_trash_destination(&rel_under_lumik, now).map_err(|e| MirrorError::Io(e.to_string()))
}

/// Ruta (relativa a `lumik/`) a la que vuelve un proyecto de la papelera; con
/// `new_name`, con ese nombre.
#[uniffi::export]
pub fn project_restore_rel(rel_in_trash: String, new_name: Option<String>) -> Result<String, MirrorError> {
    restore_rel(&rel_in_trash, new_name.as_deref()).map_err(MirrorError::Io)
}

/// Cuándo se mandó a la papelera (`AAAA-MM-DDTHH:MM:SS`), leído de su ruta
/// relativa a `lumik/` (`_trash/{fecha}/...` o, del formato anterior, `..._{fecha}`).
#[uniffi::export]
pub fn project_trashed_at(rel_in_trash: String) -> Option<String> {
    parse_trashed_at(&rel_in_trash).map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string())
}

/// Restaura una foto; con `new_stem`, todos sus archivos vuelven con ese nombre
/// base. Si el destino está ocupado falla con `restore_collision:{archivo}`.
#[uniffi::export]
pub fn restore_photo_as(
    mirror_dir: String,
    photo_id: String,
    disk: Arc<dyn DiskFiles>,
    new_stem: Option<String>,
) -> Result<(), MirrorError> {
    let db = open(&mirror_dir)?;
    let files = SafFileStore(disk);
    RestorePhoto { photos: &db, files: &files, finder_tags: &NoFinderTags, project_dir: Path::new(DISK_ROOT) }
        .execute_as(&photo_id, new_stem.as_deref())
        .map_err(MirrorError::Db)
}

/// El primer archivo que ocuparía el lugar al restaurar (con `new_stem` o con su
/// nombre), o `None` si está libre. Solo lee.
#[uniffi::export]
pub fn check_photo_restore(
    mirror_dir: String,
    photo_id: String,
    disk: Arc<dyn DiskFiles>,
    new_stem: Option<String>,
) -> Result<Option<String>, MirrorError> {
    let db = open(&mirror_dir)?;
    let files = SafFileStore(disk);
    RestorePhoto { photos: &db, files: &files, finder_tags: &NoFinderTags, project_dir: Path::new(DISK_ROOT) }
        .check(&photo_id, new_stem.as_deref())
        .map_err(MirrorError::Db)
}

/// Elimina definitivamente una foto de la papelera: sus archivos del disco y su
/// registro en la copia de trabajo.
#[uniffi::export]
pub fn purge_photo(mirror_dir: String, photo_id: String, disk: Arc<dyn DiskFiles>) -> Result<(), MirrorError> {
    let db = open(&mirror_dir)?;
    let files = SafFileStore(disk);
    PurgePhoto { photos: &db, files: &files, project_dir: Path::new(DISK_ROOT) }
        .execute(&photo_id)
        .map_err(MirrorError::Db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_match_the_desktop() {
        assert_eq!(
            project_trash_rel("2026/09/18_Risespray".into(), "2026-11-10 19:54:03".into()).unwrap(),
            "_trash/2026-11-10_19-54-03/2026/09/18_Risespray"
        );
        assert_eq!(
            project_restore_rel("_trash/2026-11-10_19-54-03/2026/09/18_Risespray".into(), None).unwrap(),
            "2026/09/18_Risespray"
        );
        assert_eq!(
            project_trashed_at("_trash/2026-11-10_19-54-03/2026/09/18_Risespray".into()).as_deref(),
            Some("2026-11-10T19:54:03")
        );
        // Formato anterior (`{ruta}__{fecha}`): se sigue pudiendo restaurar.
        assert_eq!(
            project_restore_rel("_trash/2026/09/18_Risespray__2026-11-10_19-54-03".into(), None).unwrap(),
            "2026/09/18_Risespray"
        );
        assert_eq!(
            project_restore_rel("_trash/2026/09/18_Risespray__2026-11-10_19-54-03".into(), Some("Risespray 2".into())).unwrap(),
            "2026/09/18_Risespray 2"
        );
        assert_eq!(project_trashed_at("_trash/18_X__2026-11-10_19-54-03".into()).as_deref(), Some("2026-11-10T19:54:03"));
        assert!(project_trash_rel("x".into(), "ayer".into()).is_err());
    }

    /// Papelera → restaurar con otro nombre → papelera → eliminar, sobre la copia
    /// de trabajo y un disco en memoria: no quedan archivos ni registro.
    #[test]
    fn restore_as_and_purge_on_the_working_copy() {
        use crate::disk::tests::MemDisk;
        use lumik_core::db::models::CreatePhoto;
        let work = tempfile::tempdir().unwrap();
        let mirror = work.path().display().to_string();
        crate::project::create_project_db(mirror.clone(), "Boda".into(), None, Some("2026-10-30".into()), "c".into(), "none".into()).unwrap();
        let db = open(&mirror).unwrap();
        let id = db
            .create_photos_batch(&[CreatePhoto { project_id: db.project_id.clone(), dng_path: "_media/IMG.cr2".into(), ..Default::default() }])
            .unwrap()[0]
            .id
            .clone();
        drop(db);
        let mem = Arc::new(MemDisk::default());
        mem.0.lock().unwrap().extend(["_media/IMG.cr2".to_string(), format!(".thumbs/{id}.jpg")]);
        let disk: Arc<dyn DiskFiles> = mem.clone();

        crate::disk::trash_photo(mirror.clone(), id.clone(), disk.clone()).unwrap();
        mem.0.lock().unwrap().push("_media/IMG.cr2".into());
        assert_eq!(check_photo_restore(mirror.clone(), id.clone(), disk.clone(), None).unwrap().as_deref(), Some("IMG.cr2"));
        assert!(restore_photo_as(mirror.clone(), id.clone(), disk.clone(), None).unwrap_err().to_string().starts_with("restore_collision:"));
        restore_photo_as(mirror.clone(), id.clone(), disk.clone(), Some("IMG_b".into())).unwrap();
        assert_eq!(open(&mirror).unwrap().get_photo(&id).unwrap().unwrap().dng_path, "_media/IMG_b.cr2");

        crate::disk::trash_photo(mirror.clone(), id.clone(), disk.clone()).unwrap();
        purge_photo(mirror.clone(), id.clone(), disk.clone()).unwrap();
        assert!(open(&mirror).unwrap().get_photo(&id).unwrap().is_none());
        assert_eq!(*mem.0.lock().unwrap(), vec!["_media/IMG.cr2".to_string()], "solo queda el archivo ajeno");
    }
}
