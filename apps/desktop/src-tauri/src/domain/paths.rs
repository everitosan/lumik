//! Reglas de rutas relativas dentro de un proyecto.
//! `_media/` y `_culled/` son hermanas bajo el directorio del proyecto; al
//! cullear/descullear una foto cambia solo esa carpeta, conservando el archivo.
//! Los `dng_path` se guardan siempre con separador `/` (ver `path_to_slash`).

use super::error::DomainError;

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
}
