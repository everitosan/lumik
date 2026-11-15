//! Acciones en lote sobre elementos marcados (change `photo-multi-select`):
//! mandar varias fotos a la papelera, y restaurar o eliminar definitivamente
//! varios elementos de la sección "Papelera". Cada elemento pasa por su caso de
//! uso de siempre (con su rollback); el lote solo los encadena, sigue si uno
//! falla y devuelve un reporte por elemento. Ver `docs/useCases/flujo-marcar-fotos.md`.
use super::trash::{purge_trashed_project, restore_project_inner, writable_mount};
use super::AppState;
use crate::application::error::{AppError, AppResult};
use crate::application::ports::{FileStore, FinderTagWriter};
use crate::application::use_cases::trash_photo::{PurgePhoto, RestorePhoto, TrashPhoto, RESTORE_COLLISION};
use crate::db::ProjectDatabase;
use log::info;
use std::collections::BTreeMap;

/// Un elemento que no se pudo procesar.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct BatchFailure {
    pub id: String,
    /// Nombre para mostrar (archivo o proyecto).
    pub name: String,
    /// `collision` si el lugar está ocupado, `project_locked:{dispositivo}` si el
    /// proyecto lo tiene otro, o el mensaje del error.
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct BatchReport {
    pub done: Vec<String>,
    pub failed: Vec<BatchFailure>,
}

impl BatchReport {
    fn fail(&mut self, id: &str, name: &str, reason: String) {
        self.failed.push(BatchFailure { id: id.to_string(), name: name.to_string(), reason });
    }
    fn merge(&mut self, other: BatchReport) {
        self.done.extend(other.done);
        self.failed.extend(other.failed);
    }
}

/// Un elemento de la papelera, como lo identifica la sección "Papelera".
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrashItemRef {
    Photo { project_id: String, photo_id: String },
    Project { device_uuid: String, rel_path: String },
}

/// `collision` para el error de lugar ocupado; el resto, tal cual.
fn reason_of(error: String) -> String {
    if error.starts_with(RESTORE_COLLISION) {
        "collision".to_string()
    } else {
        error
    }
}

fn photo_name(db: &ProjectDatabase, photo_id: &str) -> String {
    match db.get_photo(photo_id) {
        Ok(Some(p)) => p.dng_path.rsplit('/').next().unwrap_or(&p.dng_path).to_string(),
        _ => photo_id.to_string(),
    }
}

// ---- Lotes de fotos de un proyecto (sin `AppState`, para probarlos) ------------

/// Manda a la papelera las fotos `ids` de un proyecto, una por una.
pub(crate) fn trash_photos_in(
    db: &ProjectDatabase,
    files: &dyn FileStore,
    finder_tags: &dyn FinderTagWriter,
    ids: &[String],
) -> BatchReport {
    let mut report = BatchReport::default();
    for id in ids {
        let name = photo_name(db, id);
        match (TrashPhoto { photos: db, files, finder_tags, project_dir: &db.project_dir }).execute(id) {
            Ok(()) => report.done.push(id.clone()),
            Err(e) => report.fail(id, &name, e),
        }
    }
    report
}

/// Restaura de la papelera las fotos `ids` de un proyecto, con su nombre. Las
/// que chocan se quedan en la papelera (`collision`).
pub(crate) fn restore_photos_in(
    db: &ProjectDatabase,
    files: &dyn FileStore,
    finder_tags: &dyn FinderTagWriter,
    ids: &[String],
) -> BatchReport {
    let mut report = BatchReport::default();
    for id in ids {
        let name = photo_name(db, id);
        match (RestorePhoto { photos: db, files, finder_tags, project_dir: &db.project_dir }).execute(id) {
            Ok(()) => report.done.push(id.clone()),
            Err(e) => report.fail(id, &name, reason_of(e)),
        }
    }
    report
}

/// Elimina definitivamente las fotos `ids` de un proyecto.
pub(crate) fn purge_photos_in(db: &ProjectDatabase, files: &dyn FileStore, ids: &[String]) -> BatchReport {
    let mut report = BatchReport::default();
    for id in ids {
        let name = photo_name(db, id);
        match (PurgePhoto { photos: db, files, project_dir: &db.project_dir }).execute(id) {
            Ok(()) => report.done.push(id.clone()),
            Err(e) => report.fail(id, &name, e),
        }
    }
    report
}

// ---- Comandos ----------------------------------------------------------------

/// Manda a la papelera las fotos marcadas de un proyecto, con una sola sesión
/// (un candado y una subida de generación para todo el lote).
#[tauri::command]
pub async fn trash_photos(
    state: tauri::State<'_, AppState>,
    project_id: String,
    photo_ids: Vec<String>,
) -> AppResult<BatchReport> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {
        let session = state.write_session(&project_id)?;
        let db = session.db()?;
        let report = trash_photos_in(&db, state.file_store.as_ref(), state.finder_tags.as_ref(), &photo_ids);
        info!("trash_photos: project={} ok={} failed={}", project_id, report.done.len(), report.failed.len());
        Ok(report)
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Agrupa por proyecto las fotos de `items` y corre `per_project` con una sesión
/// por proyecto; si la sesión no se puede abrir (candado ajeno), todas sus
/// fotos fallan con ese motivo.
fn per_project_photos(
    state: &AppState,
    items: &[TrashItemRef],
    report: &mut BatchReport,
    per_project: impl Fn(&ProjectDatabase, &[String]) -> BatchReport,
) {
    let mut by_project: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for item in items {
        if let TrashItemRef::Photo { project_id, photo_id } = item {
            by_project.entry(project_id.as_str()).or_default().push(photo_id.clone());
        }
    }
    for (project_id, ids) in by_project {
        let session = match state.write_session(project_id) {
            Ok(s) => s,
            Err(e) => {
                let reason = e.to_string();
                let db = state.registry.get(project_id);
                for id in &ids {
                    let name = db.as_deref().map(|d| photo_name(d, id)).unwrap_or_else(|| id.clone());
                    report.fail(id, &name, reason.clone());
                }
                continue;
            }
        };
        match session.db() {
            Ok(db) => report.merge(per_project(&db, &ids)),
            Err(e) => {
                for id in &ids {
                    report.fail(id, id, e.clone());
                }
            }
        }
    }
}

/// Nombre de un proyecto de la papelera para el reporte: la carpeta de su ruta original.
fn project_name(rel_path: &str) -> String {
    let original = lumik_core::domain::paths::project_restore_destination(rel_path)
        .unwrap_or_else(|_| rel_path.to_string());
    original.rsplit('/').next().unwrap_or(&original).to_string()
}

/// Restaura los elementos marcados de la papelera. Ninguno abre el modal de
/// colisión: los que chocan se quedan y se reportan con `collision`.
#[tauri::command]
pub async fn restore_trash_items(state: tauri::State<'_, AppState>, items: Vec<TrashItemRef>) -> AppResult<BatchReport> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut report = BatchReport::default();
        for item in &items {
            if let TrashItemRef::Project { device_uuid, rel_path } = item {
                match restore_project_inner(&state, device_uuid, rel_path, None) {
                    Ok(_) => report.done.push(rel_path.clone()),
                    Err(e) => report.fail(rel_path, &project_name(rel_path), reason_of(e.to_string())),
                }
            }
        }
        let (files, tags) = (state.file_store.clone(), state.finder_tags.clone());
        per_project_photos(&state, &items, &mut report, |db, ids| restore_photos_in(db, files.as_ref(), tags.as_ref(), ids));
        info!("restore_trash_items: ok={} failed={}", report.done.len(), report.failed.len());
        Ok(report)
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

/// Elimina definitivamente los elementos marcados de la papelera; sigue si uno falla.
#[tauri::command]
pub async fn purge_trash_items(state: tauri::State<'_, AppState>, items: Vec<TrashItemRef>) -> AppResult<BatchReport> {
    let state = (*state).clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut report = BatchReport::default();
        for item in &items {
            if let TrashItemRef::Project { device_uuid, rel_path } = item {
                let result = writable_mount(&state, device_uuid)
                    .map_err(|e| e.to_string())
                    .and_then(|mount| purge_trashed_project(&state, &mount, device_uuid, rel_path));
                match result {
                    Ok(()) => report.done.push(rel_path.clone()),
                    Err(e) => report.fail(rel_path, &project_name(rel_path), e),
                }
            }
        }
        let files = state.file_store.clone();
        per_project_photos(&state, &items, &mut report, |db, ids| purge_photos_in(db, files.as_ref(), ids));
        info!("purge_trash_items: ok={} failed={}", report.done.len(), report.failed.len());
        Ok(report)
    })
    .await
    .map_err(|e| AppError::from(format!("tarea de fondo: {e}")))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn items_deserialize_with_kind() {
        let items: Vec<TrashItemRef> = serde_json::from_str(
            r#"[{"kind":"photo","project_id":"p","photo_id":"f"},{"kind":"project","device_uuid":"d","rel_path":"_trash/x__2026-11-10_08-00-00"}]"#,
        )
        .unwrap();
        assert!(matches!(&items[0], TrashItemRef::Photo { photo_id, .. } if photo_id == "f"));
        assert!(matches!(&items[1], TrashItemRef::Project { device_uuid, .. } if device_uuid == "d"));
    }

    #[test]
    fn names_and_reasons() {
        assert_eq!(project_name("_trash/2026-11-10_19-54-03/2026/09/18_Risespray"), "18_Risespray");
        assert_eq!(project_name("_trash/2026/09/18_Risespray__2026-11-10_19-54-03"), "18_Risespray");
        assert_eq!(reason_of(format!("{}IMG.cr2", RESTORE_COLLISION)), "collision");
        assert_eq!(reason_of("project_locked:Tablet".into()), "project_locked:Tablet");
    }
}
