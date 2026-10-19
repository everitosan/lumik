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

#[cfg(test)]
mod tests {
    use super::*;

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
