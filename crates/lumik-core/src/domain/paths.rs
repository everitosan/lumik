//! Reglas de rutas relativas dentro de un proyecto.
//! `_media/` y `_culled/` son hermanas bajo el directorio del proyecto; al
//! cullear/descullear una foto cambia solo esa carpeta, conservando el archivo.
//! Una foto en la papelera (borrado lógico) se aparta en `_trash/`, y su exportado
//! en `_trash/_exported/` para no chocar con un original JPG del mismo nombre.
//! Los `dng_path` se guardan siempre con separador `/` (ver `path_to_slash`).

use super::error::DomainError;
use std::path::Path;

/// Serializa un `Path` a string con separador `/`, para que `dng_path` en la BD
/// sea portable entre Linux, macOS y Windows.
pub fn path_to_slash(path: &Path) -> String {
    path.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Calcula el `dng_path` relativo de destino al cullear (`culled = true`) o
/// descullear (`culled = false`) una foto, a partir de su ruta relativa actual.
///
/// Equivale a la regla previa en `save_photo_culled`: subir dos niveles desde el
/// archivo (saltando `_media`/`_culled` y el nombre) para obtener el directorio
/// del proyecto, y recolocar el archivo bajo la carpeta destino.
pub fn cull_destination(current_rel: &str, culled: bool) -> Result<String, DomainError> {
    // Los paths se almacenan con '/'; normalizamos '\' por robustez.
    let normalized = current_rel.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();

    if parts.is_empty() {
        return Err(DomainError::InvalidPhotoPath(
            "dng_path sin nombre de archivo".to_string(),
        ));
    }
    let filename = parts[parts.len() - 1];

    // Hace falta al menos <_media|_culled>/<archivo> para ubicar el proyecto.
    if parts.len() < 2 {
        return Err(DomainError::InvalidPhotoPath(
            "No se puede determinar el directorio del proyecto".to_string(),
        ));
    }

    let project_dir_rel = &parts[..parts.len() - 2];
    let subdir = if culled { "_culled" } else { "_media" };

    let mut out: Vec<&str> = project_dir_rel.to_vec();
    out.push(subdir);
    out.push(filename);
    Ok(out.join("/"))
}

/// Divide una ruta relativa en (directorio del proyecto, nombre de archivo),
/// sabiendo que el archivo está `levels` carpetas por debajo del proyecto
/// (`_media/x` → 1, `_trash/_exported/x` → 2).
fn split_project(current_rel: &str, levels: usize) -> Result<(Vec<&str>, &str), DomainError> {
    let parts: Vec<&str> = current_rel.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Err(DomainError::InvalidPhotoPath(
            "ruta sin nombre de archivo".to_string(),
        ));
    }
    if parts.len() < levels + 1 {
        return Err(DomainError::InvalidPhotoPath(
            "No se puede determinar el directorio del proyecto".to_string(),
        ));
    }
    let filename = parts[parts.len() - 1];
    Ok((parts[..parts.len() - 1 - levels].to_vec(), filename))
}

fn join_rel(project: &[&str], subdirs: &[&str], filename: &str) -> String {
    let mut out: Vec<&str> = project.to_vec();
    out.extend_from_slice(subdirs);
    out.push(filename);
    out.join("/")
}

/// Destino de un archivo del proyecto al entregarlo: `{proyecto}/_exported/{nombre}`,
/// venga de `_media/` o de `_culled/`.
pub fn exported_destination(current_rel: &str) -> Result<String, DomainError> {
    let normalized = current_rel.replace('\\', "/");
    let (project, filename) = split_project(&normalized, 1)?;
    Ok(join_rel(&project, &["_exported"], filename))
}

/// ¿Está el archivo directamente en la subcarpeta `subdir` del proyecto?
pub fn is_in_subdir(rel: &str, subdir: &str) -> bool {
    let normalized = rel.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    parts.len() >= 2 && parts[parts.len() - 2] == subdir
}

/// Destino del archivo original al mover una foto a la papelera: `{proyecto}/_trash/{nombre}`,
/// venga de `_media/` o de `_culled/`.
pub fn trash_destination(current_rel: &str) -> Result<String, DomainError> {
    let normalized = current_rel.replace('\\', "/");
    let (project, filename) = split_project(&normalized, 1)?;
    Ok(join_rel(&project, &["_trash"], filename))
}

/// Destino del JPG exportado al mover su foto a la papelera:
/// `{proyecto}/_exported/{nombre}` → `{proyecto}/_trash/_exported/{nombre}`.
pub fn trash_exported_destination(jpg_rel: &str) -> Result<String, DomainError> {
    let normalized = jpg_rel.replace('\\', "/");
    let (project, filename) = split_project(&normalized, 1)?;
    Ok(join_rel(&project, &["_trash", "_exported"], filename))
}

/// Destino del original al restaurar una foto de la papelera: `_culled/` si estaba
/// aprobada, `_media/` si no.
pub fn restore_destination(trashed_rel: &str, culled: bool) -> Result<String, DomainError> {
    let normalized = trashed_rel.replace('\\', "/");
    let (project, filename) = split_project(&normalized, 1)?;
    let subdir = if culled { "_culled" } else { "_media" };
    Ok(join_rel(&project, &[subdir], filename))
}

/// Destino del exportado al restaurar:
/// `{proyecto}/_trash/_exported/{nombre}` → `{proyecto}/_exported/{nombre}`.
pub fn restore_exported_destination(trashed_jpg_rel: &str) -> Result<String, DomainError> {
    let normalized = trashed_jpg_rel.replace('\\', "/");
    let (project, filename) = split_project(&normalized, 2)?;
    Ok(join_rel(&project, &["_exported"], filename))
}

/// Ruta esperada en `_trash/_exported/` de un exportado llamado `jpg_filename`,
/// tomando el proyecto de la ruta del original en la papelera (`{proyecto}/_trash/x`).
/// Sirve de alternativa cuando el `jpg_path` guardado ya no existe.
pub fn trashed_exported_for(trashed_rel: &str, jpg_filename: &str) -> Result<String, DomainError> {
    let normalized = trashed_rel.replace('\\', "/");
    let (project, _) = split_project(&normalized, 1)?;
    Ok(join_rel(&project, &["_trash", "_exported"], jpg_filename))
}

// ============================================================================
// PAPELERA DE PROYECTOS
// Un proyecto está en la papelera si y solo si su carpeta está bajo
// `lumik/_trash/`. Va a `lumik/_trash/{AAAA-MM-DD_HH-MM-SS}/{ruta}` (fecha y hora
// locales en que se mandó; guiones: exFAT no acepta `:`). Como el nombre de la
// carpeta no cambia, mandarla y restaurarla es un solo movimiento: en Android
// cada operación sobre una carpeta cuesta tiempo por cada archivo que contiene.
//
// Formato anterior, que se sigue leyendo y restaurando:
// `lumik/_trash/{ruta}__{AAAA-MM-DD_HH-MM-SS}`.
// Las rutas de estas funciones son relativas a `lumik/`, con `/`.
// ============================================================================

/// Carpeta de la papelera de proyectos, relativa a `lumik/`.
pub const PROJECT_TRASH_DIR: &str = "_trash";

const TRASHED_AT_FORMAT: &str = "%Y-%m-%d_%H-%M-%S";
/// Largo de `AAAA-MM-DD_HH-MM-SS`.
const TRASHED_AT_LEN: usize = 19;

fn rel_segments(rel: &str) -> Vec<&str> {
    rel.split(['/', '\\']).filter(|s| !s.is_empty() && *s != ".").collect()
}

fn parse_stamp(s: &str) -> Option<chrono::NaiveDateTime> {
    if s.len() != TRASHED_AT_LEN {
        return None;
    }
    chrono::NaiveDateTime::parse_from_str(s, TRASHED_AT_FORMAT).ok()
}

/// `lumik/{ruta}` → `lumik/_trash/{fecha}/{ruta}`, con la fecha en que se manda.
pub fn project_trash_destination(rel_under_lumik: &str, now: chrono::NaiveDateTime) -> Result<String, DomainError> {
    let parts = rel_segments(rel_under_lumik);
    if parts.is_empty() || parts.contains(&"..") {
        return Err(DomainError::InvalidPhotoPath(format!("Ruta de proyecto inválida: {}", rel_under_lumik)));
    }
    if parts[0] == PROJECT_TRASH_DIR {
        return Err(DomainError::InvalidPhotoPath("El proyecto ya está en la papelera".to_string()));
    }
    let stamp = now.format(TRASHED_AT_FORMAT).to_string();
    let mut out = vec![PROJECT_TRASH_DIR, stamp.as_str()];
    out.extend(parts);
    Ok(out.join("/"))
}

/// Separa el sufijo de fecha del formato anterior (`{nombre}__{fecha}`), si lo tiene.
fn split_trashed_suffix(name: &str) -> (&str, Option<chrono::NaiveDateTime>) {
    if let Some(idx) = name.rfind("__") {
        if let Some(dt) = parse_stamp(&name[idx + 2..]) {
            if idx > 0 {
                return (&name[..idx], Some(dt));
            }
        }
    }
    (name, None)
}

/// Ruta original y fecha de un elemento de la papelera (`_trash/...`), en
/// cualquiera de los dos formatos. `None` si no queda dentro de la papelera.
fn split_trashed(rel_in_trash: &str) -> Option<(Vec<&str>, Option<chrono::NaiveDateTime>)> {
    let parts = rel_segments(rel_in_trash);
    if parts.len() < 2 || parts[0] != PROJECT_TRASH_DIR || parts.contains(&"..") {
        return None;
    }
    if parts.len() >= 3 {
        if let Some(dt) = parse_stamp(parts[1]) {
            return Some((parts[2..].to_vec(), Some(dt)));
        }
    }
    let mut out: Vec<&str> = parts[1..].to_vec();
    let last = out.pop().unwrap();
    let (name, dt) = split_trashed_suffix(last);
    out.push(name);
    Some((out, dt))
}

/// Fecha en que se mandó a la papelera, leída de la ruta del elemento
/// (`_trash/{fecha}/...` o, del formato anterior, `_trash/.../{nombre}__{fecha}`).
pub fn parse_trashed_at(rel_in_trash: &str) -> Option<chrono::NaiveDateTime> {
    split_trashed(rel_in_trash)?.1
}

/// `lumik/_trash/{fecha}/{ruta}` (o `_trash/{ruta}__{fecha}`) → `lumik/{ruta}`.
/// Sin fecha reconocible solo se quita `_trash/`.
pub fn project_restore_destination(rel_in_trash: &str) -> Result<String, DomainError> {
    split_trashed(rel_in_trash)
        .map(|(parts, _)| parts.join("/"))
        .ok_or_else(|| DomainError::InvalidPhotoPath(format!("No está en la papelera: {}", rel_in_trash)))
}

/// ¿Queda `path` dentro de `{mount}/lumik/_trash/` (y no es esa carpeta)?
/// Guarda de los borrados definitivos de proyectos: nunca se borra uno activo.
/// Compara componentes léxicos y rechaza `..`, así que no depende de que la ruta exista.
pub fn is_inside_project_trash(mount_point: &Path, path: &Path) -> bool {
    use std::path::Component;
    if path.components().any(|c| matches!(c, Component::ParentDir)) {
        return false;
    }
    let trash = mount_point.join("lumik").join(PROJECT_TRASH_DIR);
    match path.strip_prefix(&trash) {
        Ok(rest) => rest.components().any(|c| matches!(c, Component::Normal(_))),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> chrono::NaiveDateTime {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    // ---- Papelera de proyectos ----------------------------------------------

    #[test]
    fn project_trash_by_date_structure() {
        assert_eq!(
            project_trash_destination("2026/09/18_Risespray", at("2026-11-10 19:54:03")).unwrap(),
            "_trash/2026-11-10_19-54-03/2026/09/18_Risespray"
        );
    }

    #[test]
    fn project_trash_keeps_folder_name() {
        // Un solo movimiento: el nombre de la carpeta no cambia.
        let dest = project_trash_destination("2026/09/18_X", at("2026-11-10 08:00:00")).unwrap();
        assert_eq!(dest.rsplit('/').next(), Some("18_X"));
    }

    #[test]
    fn project_trash_flat_path() {
        assert_eq!(
            project_trash_destination("Emma Arenas", at("2026-11-10 08:00:00")).unwrap(),
            "_trash/2026-11-10_08-00-00/Emma Arenas"
        );
    }

    #[test]
    fn project_trash_rejects_bad_paths() {
        assert!(project_trash_destination("", at("2026-11-10 08:00:00")).is_err());
        assert!(project_trash_destination("../x", at("2026-11-10 08:00:00")).is_err());
        assert!(project_trash_destination("_trash/2026-11-10_08-00-00/x", at("2026-11-10 08:00:00")).is_err());
    }

    #[test]
    fn project_trash_differs_by_second() {
        let a = project_trash_destination("2026/09/18_X", at("2026-11-10 08:00:00")).unwrap();
        let b = project_trash_destination("2026/09/18_X", at("2026-11-10 08:00:01")).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn restore_strips_trash_and_date() {
        let trashed = "_trash/2026-11-10_19-54-03/2026/09/18_Risespray";
        assert_eq!(project_restore_destination(trashed).unwrap(), "2026/09/18_Risespray");
        assert_eq!(parse_trashed_at(trashed), Some(at("2026-11-10 19:54:03")));
        assert_eq!(project_restore_destination("_trash/2026-11-10_08-00-00/Emma Arenas").unwrap(), "Emma Arenas");
    }

    #[test]
    fn restore_reads_previous_format() {
        let old = "_trash/2026/09/18_Risespray__2026-11-10_19-54-03";
        assert_eq!(project_restore_destination(old).unwrap(), "2026/09/18_Risespray");
        assert_eq!(parse_trashed_at(old), Some(at("2026-11-10 19:54:03")));
        assert_eq!(project_restore_destination("_trash/Emma Arenas__2026-11-10_08-00-00").unwrap(), "Emma Arenas");
    }

    #[test]
    fn restore_without_valid_date_only_strips_trash() {
        assert_eq!(project_restore_destination("_trash/2026/09/18_Boda").unwrap(), "2026/09/18_Boda");
        assert_eq!(parse_trashed_at("_trash/2026/09/18_Boda"), None);
        assert_eq!(project_restore_destination("_trash/18_Boda__2026-13-40_99-00-00").unwrap(), "18_Boda__2026-13-40_99-00-00");
        // Una carpeta con forma de fecha como único segmento es la ruta, no la fecha.
        assert_eq!(project_restore_destination("_trash/2026-11-10_08-00-00").unwrap(), "2026-11-10_08-00-00");
    }

    #[test]
    fn name_with_double_underscore_round_trips() {
        let trashed = project_trash_destination("2026/01/05_Boda__Ana", at("2026-11-10 08:00:00")).unwrap();
        assert_eq!(trashed, "_trash/2026-11-10_08-00-00/2026/01/05_Boda__Ana");
        assert_eq!(project_restore_destination(&trashed).unwrap(), "2026/01/05_Boda__Ana");
    }

    #[test]
    fn restore_rejects_paths_outside_trash() {
        assert!(project_restore_destination("2026/09/18_X").is_err());
        assert!(project_restore_destination("_trash").is_err());
        assert!(project_restore_destination("_trash/../2026/x").is_err());
        assert!(project_restore_destination("_trash/2026-11-10_08-00-00/../x").is_err());
    }

    #[test]
    fn trash_guard() {
        let mount = Path::new("/media/u/KINGSTON");
        assert!(is_inside_project_trash(mount, &mount.join("lumik/_trash/2026/09/18_X__2026-11-10_08-00-00")));
        assert!(!is_inside_project_trash(mount, &mount.join("lumik/2026/09/18_X")));
        assert!(!is_inside_project_trash(mount, &mount.join("lumik/_trash")));
        assert!(!is_inside_project_trash(mount, &mount.join("lumik/_trash/../2026/x")));
        assert!(!is_inside_project_trash(mount, Path::new("/home/u/lumik/_trash/x")));
    }

    #[test]
    fn exported_destination_and_subdir() {
        assert_eq!(exported_destination("_culled/IMG.jpg").unwrap(), "_exported/IMG.jpg");
        assert_eq!(exported_destination("lumik/x/_media/IMG.jpg").unwrap(), "lumik/x/_exported/IMG.jpg");
        assert!(is_in_subdir("_exported/IMG.jpg", "_exported"));
        assert!(!is_in_subdir("_media/IMG.jpg", "_exported"));
    }

    #[test]
    fn cull_moves_media_to_culled() {
        assert_eq!(
            cull_destination("lumik/2024/06/15_boda/_media/IMG_0001.dng", true).unwrap(),
            "lumik/2024/06/15_boda/_culled/IMG_0001.dng"
        );
    }

    #[test]
    fn uncull_moves_culled_to_media() {
        assert_eq!(
            cull_destination("lumik/2024/06/15_boda/_culled/IMG_0001.dng", false).unwrap(),
            "lumik/2024/06/15_boda/_media/IMG_0001.dng"
        );
    }

    #[test]
    fn preserves_filename_and_project_dir() {
        assert_eq!(
            cull_destination("lumik/PlatziConf/_media/foto final.dng", true).unwrap(),
            "lumik/PlatziConf/_culled/foto final.dng"
        );
    }

    #[test]
    fn minimal_path_without_project_dir() {
        // _media/archivo → _culled/archivo (directorio de proyecto vacío)
        assert_eq!(
            cull_destination("_media/IMG.dng", true).unwrap(),
            "_culled/IMG.dng"
        );
    }

    #[test]
    fn bare_filename_errors() {
        assert!(cull_destination("IMG.dng", true).is_err());
        assert!(cull_destination("", true).is_err());
    }

    // ---- Papelera / restaurar ------------------------------------------------

    #[test]
    fn trash_moves_media_and_culled_to_trash() {
        assert_eq!(
            trash_destination("lumik/2024/06/15_boda/_media/IMG_0001.dng").unwrap(),
            "lumik/2024/06/15_boda/_trash/IMG_0001.dng"
        );
        assert_eq!(
            trash_destination("lumik/2024/06/15_boda/_culled/IMG_0002.cr2").unwrap(),
            "lumik/2024/06/15_boda/_trash/IMG_0002.cr2"
        );
    }

    #[test]
    fn trash_exported_goes_to_trash_exported() {
        assert_eq!(
            trash_exported_destination("lumik/2024/06/15_boda/_exported/IMG_0001.jpg").unwrap(),
            "lumik/2024/06/15_boda/_trash/_exported/IMG_0001.jpg"
        );
    }

    #[test]
    fn jpg_original_and_export_with_same_name_do_not_collide() {
        let original = trash_destination("p/_media/IMG_0003.jpg").unwrap();
        let exported = trash_exported_destination("p/_exported/IMG_0003.jpg").unwrap();
        assert_eq!(original, "p/_trash/IMG_0003.jpg");
        assert_eq!(exported, "p/_trash/_exported/IMG_0003.jpg");
        assert_ne!(original, exported);
    }

    #[test]
    fn restore_goes_back_according_to_culled() {
        assert_eq!(
            restore_destination("lumik/PlatziConf/_trash/foto final.dng", true).unwrap(),
            "lumik/PlatziConf/_culled/foto final.dng"
        );
        assert_eq!(
            restore_destination("lumik/PlatziConf/_trash/foto final.dng", false).unwrap(),
            "lumik/PlatziConf/_media/foto final.dng"
        );
    }

    #[test]
    fn restore_exported_goes_back_to_exported() {
        assert_eq!(
            restore_exported_destination("lumik/2024/06/15_boda/_trash/_exported/IMG_0001.jpg").unwrap(),
            "lumik/2024/06/15_boda/_exported/IMG_0001.jpg"
        );
    }

    #[test]
    fn trashed_exported_for_uses_photo_project() {
        assert_eq!(
            trashed_exported_for("lumik/2024/06/15_boda/_trash/IMG_0001.cr2", "IMG_0001.jpg").unwrap(),
            "lumik/2024/06/15_boda/_trash/_exported/IMG_0001.jpg"
        );
    }

    #[test]
    fn trash_and_restore_normalize_backslashes() {
        assert_eq!(
            trash_destination("lumik\\Boda\\_media\\IMG.dng").unwrap(),
            "lumik/Boda/_trash/IMG.dng"
        );
    }

    #[test]
    fn trash_paths_without_project_dir_error() {
        assert!(trash_destination("IMG.dng").is_err());
        assert!(trash_destination("").is_err());
        assert!(restore_exported_destination("_exported/IMG.jpg").is_err());
    }
}
