//! Papelera de proyectos sobre el sistema de archivos local (spec `trash-bin`,
//! `project-management`). Un proyecto está en la papelera si y solo si su carpeta
//! está bajo `{mount}/lumik/_trash/`; las rutas salen de `domain::paths`.
//!
//! Lo usa el desktop. La tablet hace los mismos movimientos por SAF con las
//! mismas reglas de `domain::paths`.

use crate::db::ProjectDatabase;
use crate::domain::lock::LOCK_FILE;
use crate::domain::paths::{
    is_inside_project_trash, parse_trashed_at, path_to_slash, project_restore_destination,
    project_trash_destination, PROJECT_TRASH_DIR,
};
use crate::domain::project::ProjectFolder;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Profundidad máxima bajo `lumik/_trash/`, la misma que el escaneo de `lumik/`.
const MAX_DEPTH: usize = 4;

/// Un proyecto en la papelera de un disco, como lo muestra la sección "Papelera".
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TrashedProject {
    pub id: String,
    pub name: String,
    pub session_date: Option<String>,
    pub created_at: String,
    pub photo_count: i32,
    pub workflow_status: String,
    /// Ruta relativa a `lumik/`, con `/` (`_trash/2026-11-10_19-54-03/2026/09/18_X`;
    /// del formato anterior, `_trash/2026/09/18_X__2026-11-10_19-54-03`).
    /// Identifica al elemento en los comandos de la papelera.
    pub rel_path: String,
    /// Cuándo se mandó, leído de la ruta (`AAAA-MM-DDTHH:MM:SS`, hora local); `None` sin fecha.
    pub trashed_at: Option<String>,
}

fn lumik_dir(mount_point: &Path) -> PathBuf {
    mount_point.join("lumik")
}

/// Ruta absoluta de un elemento de la papelera a partir de su `rel_path`.
/// Falla si no queda dentro de `lumik/_trash/` (guarda contra tocar un proyecto activo).
pub fn trashed_dir(mount_point: &Path, rel_path: &str) -> Result<PathBuf, String> {
    let dir = rel_path
        .split(['/', '\\'])
        .filter(|s| !s.is_empty())
        .fold(lumik_dir(mount_point), |acc, s| acc.join(s));
    if !is_inside_project_trash(mount_point, &dir) {
        return Err(format!("No está en la papelera: {}", rel_path));
    }
    Ok(dir)
}

/// Proyectos bajo `{mount}/lumik/_trash/`, del más reciente al más antiguo.
/// Abre cada `project.db` solo para leer su tarjeta y lo cierra: no se registran.
pub fn discover_trashed_projects(mount_point: &Path, device_uuid: &str) -> Vec<TrashedProject> {
    let root = lumik_dir(mount_point).join(PROJECT_TRASH_DIR);
    let mut dirs = Vec::new();
    if root.is_dir() {
        walk(&root, 0, &mut dirs);
    }
    let mut out: Vec<TrashedProject> = dirs
        .into_iter()
        .filter_map(|dir| read_trashed(mount_point, device_uuid, &dir))
        .collect();
    out.sort_by(|a, b| b.trashed_at.cmp(&a.trashed_at).then_with(|| a.rel_path.cmp(&b.rel_path)));
    out
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > MAX_DEPTH {
        return;
    }
    if dir.join("project.db").exists() {
        out.push(dir.to_path_buf());
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name.starts_with('_') {
            continue;
        }
        walk(&path, depth + 1, out);
    }
}

fn read_trashed(mount_point: &Path, device_uuid: &str, dir: &Path) -> Option<TrashedProject> {
    let db = match ProjectDatabase::open(dir.join("project.db"), device_uuid, mount_point.to_path_buf()) {
        Ok(db) => db,
        Err(e) => {
            log::warn!("papelera: no se pudo abrir {}: {}", dir.display(), e);
            return None;
        }
    };
    let entry = db.get_project_dashboard_entry().ok()??;
    let rel = path_to_slash(dir.strip_prefix(lumik_dir(mount_point)).ok()?);
    let trashed_at = parse_trashed_at(&rel).map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string());
    Some(TrashedProject {
        id: entry.id,
        name: entry.name,
        session_date: entry.session_date,
        created_at: entry.created_at,
        photo_count: entry.photo_count,
        workflow_status: entry.workflow_status,
        rel_path: rel,
        trashed_at,
    })
}

/// Carpeta en la papelera para el proyecto en `project_dir`, mandado ahora.
pub fn trash_target(mount_point: &Path, project_dir: &Path, now: chrono::NaiveDateTime) -> Result<PathBuf, String> {
    let rel = project_dir
        .strip_prefix(lumik_dir(mount_point))
        .map_err(|_| format!("El proyecto no está bajo lumik/: {}", project_dir.display()))?;
    let dest = project_trash_destination(&path_to_slash(rel), now).map_err(|e| e.to_string())?;
    Ok(dest.split('/').fold(lumik_dir(mount_point), |acc, s| acc.join(s)))
}

/// Ruta (relativa a `lumik/`, con `/`) a la que vuelve un proyecto de la
/// papelera. Sin `new_name`, su ruta original; con `new_name`, la
/// misma carpeta padre con el nombre nuevo (`{día}_{nombre}` si la original
/// seguía ese patrón). La comparten el desktop y la tablet.
pub fn restore_rel(rel_in_trash: &str, new_name: Option<&str>) -> Result<String, String> {
    let original = project_restore_destination(rel_in_trash).map_err(|e| e.to_string())?;
    let mut parts: Vec<&str> = original.split('/').collect();
    let last = parts.pop().unwrap_or_default();
    let folder = match new_name.map(str::trim) {
        None => last.to_string(),
        Some("") => return Err("El nombre no puede estar vacío".to_string()),
        Some(name) => match day_prefix(last) {
            Some(day) => ProjectFolder::folder_name(day, name),
            None => ProjectFolder::slug(name),
        },
    };
    parts.push(&folder);
    Ok(parts.join("/"))
}

/// Carpeta absoluta a la que vuelve un proyecto de la papelera (ver `restore_rel`).
pub fn restore_target(mount_point: &Path, rel_in_trash: &str, new_name: Option<&str>) -> Result<PathBuf, String> {
    let rel = restore_rel(rel_in_trash, new_name)?;
    Ok(rel.split('/').fold(lumik_dir(mount_point), |acc, s| acc.join(s)))
}

/// `18_Risespray` → `18`. Solo si empieza con dos dígitos y `_`.
fn day_prefix(folder: &str) -> Option<&str> {
    let (day, rest) = folder.split_once('_')?;
    (day.len() == 2 && day.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty()).then_some(day)
}

/// Mueve la carpeta `from` a `to` (que no debe existir) y quita las carpetas que
/// quedan vacías encima de `from`, sin subir de `stop`. Todo en el mismo disco.
pub fn move_project_dir(from: &Path, to: &Path, stop: &Path) -> Result<(), String> {
    if to.exists() {
        return Err(format!("Ya existe una carpeta en {}", to.display()));
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("No se pudo crear {}: {}", parent.display(), e))?;
    }
    std::fs::rename(from, to).map_err(|e| format!("No se pudo mover {}: {}", from.display(), e))?;
    if let Some(parent) = from.parent() {
        prune_empty_dirs(parent, stop);
    }
    Ok(())
}

/// Quita `dir` y sus padres mientras estén vacíos, sin quitar `stop` ni subir de él.
pub fn prune_empty_dirs(dir: &Path, stop: &Path) {
    let mut current = Some(dir);
    while let Some(d) = current {
        if d == stop || !d.starts_with(stop) {
            break;
        }
        if std::fs::remove_dir(d).is_err() {
            break;
        }
        current = d.parent();
    }
}

/// Quita un `.lumik-lock` que haya viajado a la papelera con el proyecto.
pub fn clear_stale_lock(project_dir: &Path) {
    let lock = project_dir.join(LOCK_FILE);
    if lock.exists() {
        if let Err(e) = std::fs::remove_file(&lock) {
            log::warn!("papelera: no se pudo quitar {}: {}", lock.display(), e);
        }
    }
}

/// Borra para siempre la carpeta de un proyecto en la papelera y las carpetas
/// vacías que deja bajo `lumik/_trash/`. Rechaza cualquier ruta fuera de la papelera.
/// Si la carpeta ya no está, no es error.
pub fn purge_project_dir(mount_point: &Path, rel_in_trash: &str) -> Result<(), String> {
    let dir = trashed_dir(mount_point, rel_in_trash)?;
    if dir.exists() {
        std::fs::remove_dir_all(&dir).map_err(|e| format!("No se pudo borrar {}: {}", dir.display(), e))?;
    }
    if let Some(parent) = dir.parent() {
        prune_empty_dirs(parent, &lumik_dir(mount_point).join(PROJECT_TRASH_DIR));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> chrono::NaiveDateTime {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    fn tmp_mount(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lumik-trash-{}-{}", tag, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(dir.join("lumik")).unwrap();
        dir
    }

    fn make_project(mount: &Path, rel: &str, name: &str) -> PathBuf {
        let dir = rel.split('/').fold(mount.join("lumik"), |a, s| a.join(s));
        crate::db::ProjectDatabase::create(
            dir.join("project.db"),
            &uuid::Uuid::new_v4().to_string(),
            name,
            "creator",
            None,
            Some("2026-09-18"),
            "dev",
            mount.to_path_buf(),
        )
        .unwrap();
        dir
    }

    #[test]
    fn trash_and_restore_round_trip() {
        let mount = tmp_mount("rt");
        let dir = make_project(&mount, "2026/09/18_Risespray", "Risespray");
        let target = trash_target(&mount, &dir, at("2026-11-10 19:54:03")).unwrap();
        assert_eq!(target, mount.join("lumik/_trash/2026-11-10_19-54-03/2026/09/18_Risespray"));

        move_project_dir(&dir, &target, &mount.join("lumik")).unwrap();
        assert!(!mount.join("lumik/2026").exists(), "las carpetas vacías de año y mes se quitan");
        assert!(crate::db::discover_projects_on_device(mount.to_str().unwrap(), "dev").is_empty());

        let trashed = discover_trashed_projects(&mount, "dev");
        assert_eq!(trashed.len(), 1);
        assert_eq!(trashed[0].name, "Risespray");
        assert_eq!(trashed[0].rel_path, "_trash/2026-11-10_19-54-03/2026/09/18_Risespray");
        assert_eq!(trashed[0].trashed_at.as_deref(), Some("2026-11-10T19:54:03"));

        let back = restore_target(&mount, &trashed[0].rel_path, None).unwrap();
        assert_eq!(back, dir);
        move_project_dir(&target, &back, &mount.join("lumik").join(PROJECT_TRASH_DIR)).unwrap();
        assert!(!mount.join("lumik/_trash/2026-11-10_19-54-03").exists(), "la carpeta de la fecha se quita");
        assert_eq!(crate::db::discover_projects_on_device(mount.to_str().unwrap(), "dev").len(), 1);
        let _ = std::fs::remove_dir_all(&mount);
    }

    #[test]
    fn move_refuses_occupied_target() {
        let mount = tmp_mount("occ");
        let a = make_project(&mount, "2026/09/18_A", "A");
        let b = make_project(&mount, "2026/09/18_B", "B");
        assert!(move_project_dir(&a, &b, &mount.join("lumik")).is_err());
        assert!(a.join("project.db").exists());
        let _ = std::fs::remove_dir_all(&mount);
    }

    #[test]
    fn restore_target_with_new_name() {
        let mount = Path::new("/m");
        assert_eq!(
            restore_target(mount, "_trash/2026-11-10_19-54-03/2026/09/18_Risespray", Some("Risespray 2")).unwrap(),
            Path::new("/m/lumik/2026/09/18_Risespray 2")
        );
        assert_eq!(
            restore_target(mount, "_trash/2026-11-10_19-54-03/Emma Arenas", Some("Emma")).unwrap(),
            Path::new("/m/lumik/Emma")
        );
        assert!(restore_target(mount, "_trash/2026/09/18_X", Some("  ")).is_err());
    }

    #[test]
    fn photos_in_trashed_project_travel_with_it() {
        let mount = tmp_mount("photos");
        let dir = make_project(&mount, "2026/09/18_X", "X");
        std::fs::create_dir_all(dir.join("_trash")).unwrap();
        std::fs::write(dir.join("_trash/IMG.cr2"), b"x").unwrap();
        let target = trash_target(&mount, &dir, at("2026-11-10 08:00:00")).unwrap();
        move_project_dir(&dir, &target, &mount.join("lumik")).unwrap();
        assert!(target.join("_trash/IMG.cr2").exists());
        assert_eq!(discover_trashed_projects(&mount, "dev").len(), 1);
        let _ = std::fs::remove_dir_all(&mount);
    }

    #[test]
    fn purge_only_inside_trash() {
        let mount = tmp_mount("purge");
        let active = make_project(&mount, "2026/09/18_A", "A");
        assert!(purge_project_dir(&mount, "2026/09/18_A").is_err());
        assert!(purge_project_dir(&mount, "_trash/../2026/09/18_A").is_err());
        assert!(active.exists());

        let dir = make_project(&mount, "2026/09/18_B", "B");
        let target = trash_target(&mount, &dir, at("2026-11-10 08:00:00")).unwrap();
        move_project_dir(&dir, &target, &mount.join("lumik")).unwrap();
        purge_project_dir(&mount, "_trash/2026-11-10_08-00-00/2026/09/18_B").unwrap();
        assert!(!mount.join("lumik/_trash/2026-11-10_08-00-00").exists(), "quita las carpetas vacías");
        // Repetirlo no falla.
        purge_project_dir(&mount, "_trash/2026-11-10_08-00-00/2026/09/18_B").unwrap();
        let _ = std::fs::remove_dir_all(&mount);
    }

    #[test]
    fn previous_format_is_listed_and_restored() {
        let mount = tmp_mount("legacy");
        let original = make_project(&mount, "2026/09/18_Old", "Old");
        let old = mount.join("lumik/_trash/2026/09/18_Old__2026-11-10_08-00-00");
        move_project_dir(&original, &old, &mount.join("lumik")).unwrap();

        let trashed = discover_trashed_projects(&mount, "dev");
        assert_eq!(trashed.len(), 1);
        assert_eq!(trashed[0].trashed_at.as_deref(), Some("2026-11-10T08:00:00"));
        let back = restore_target(&mount, &trashed[0].rel_path, None).unwrap();
        assert_eq!(back, original);
        let _ = std::fs::remove_dir_all(&mount);
    }
}
