//! Reglas de proyecto: derivación del slug y de la carpeta por fecha en disco,
//! y orden del dashboard (incluido el orden elegido por el usuario). Puras (sin reloj ni sistema de archivos); el reloj
//! (fecha de hoy como fallback) lo resuelve el llamador.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// Estructura de carpeta de un proyecto en disco:
/// `{mount}/lumik/{año}/{mes}/{día}_{slug}`.
pub struct ProjectFolder;

impl ProjectFolder {
    /// Slug del proyecto para nombre de carpeta/archivo: reemplaza `/` por `-`
    /// y recorta espacios. Coincide con el usado por `create_project`.
    pub fn slug(name: &str) -> String {
        name.replace('/', "-").trim().to_string()
    }

    /// Nombre de carpeta del proyecto: `{día}_{slug}`.
    pub fn folder_name(day: &str, name: &str) -> String {
        format!("{}_{}", day, ProjectFolder::slug(name))
    }

    /// Descompone una fecha en `(año, mes, día)`. Acepta formato ISO
    /// (`"YYYY-MM-DD..."`) y EXIF (`"YYYY:MM:DD..."`). Devuelve `None` si la
    /// cadena tiene menos de 10 caracteres o no parte en tres segmentos.
    pub fn date_parts(date: &str) -> Option<(String, String, String)> {
        if date.len() < 10 {
            return None;
        }
        let normalized = date[..10].replace(':', "-");
        let parts: Vec<&str> = normalized.splitn(3, '-').collect();
        if parts.len() != 3 || parts.iter().any(|p| p.is_empty()) {
            return None;
        }
        Some((parts[0].to_string(), parts[1].to_string(), parts[2].to_string()))
    }

    /// Ruta absoluta de la carpeta del proyecto bajo `mount`.
    pub fn path(mount: &Path, year: &str, month: &str, day: &str, name: &str) -> PathBuf {
        mount
            .join("lumik")
            .join(year)
            .join(month)
            .join(ProjectFolder::folder_name(day, name))
    }
}

/// Orden del dashboard: `session_date` descendente con NULLs al final, y como
/// desempate `created_at` descendente. Pensado para `sort_by(|a, b| ...)`.
///
/// NOTA: corrige un bug latente de la implementación previa en
/// `get_projects_dashboard`, cuyo comentario decía "NULLS LAST" pero cuyo `match`
/// (invertido sobre `(b, a)`) en realidad colocaba los proyectos sin fecha al
/// PRINCIPIO. Aquí se respeta la intención documentada: sin fecha van al final.
pub fn compare_dashboard(
    a: (Option<&str>, &str),
    b: (Option<&str>, &str),
) -> Ordering {
    match (a.0, b.0) {
        // Ambos con fecha: session_date DESC, desempate created_at DESC.
        (Some(ad), Some(bd)) => bd.cmp(ad).then(b.1.cmp(a.1)),
        // Con fecha antes que sin fecha (NULLS LAST).
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        // Ambos sin fecha: created_at DESC.
        (None, None) => b.1.cmp(a.1),
    }
}

/// Dirección de un orden del dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
        }
    }

    /// Interpreta un valor guardado; `None` si falta o es desconocido.
    pub fn parse(value: Option<&str>) -> Option<Self> {
        match value? {
            "asc" => Some(SortDirection::Asc),
            "desc" => Some(SortDirection::Desc),
            _ => None,
        }
    }
}

/// Orden elegido por el usuario para el dashboard: secciones de año y
/// proyectos dentro de cada año. Se guarda en la configuración global.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardSort {
    pub years: SortDirection,
    pub projects: SortDirection,
}

impl Default for DashboardSort {
    /// El orden histórico del dashboard: años recientes primero y, dentro de
    /// cada año, proyectos antiguos primero.
    fn default() -> Self {
        Self {
            years: SortDirection::Desc,
            projects: SortDirection::Asc,
        }
    }
}

impl DashboardSort {
    /// Construye el orden desde los valores guardados; cada clave ausente o
    /// desconocida toma su valor por defecto.
    pub fn from_stored(years: Option<&str>, projects: Option<&str>) -> Self {
        let default = Self::default();
        Self {
            years: SortDirection::parse(years).unwrap_or(default.years),
            projects: SortDirection::parse(projects).unwrap_or(default.projects),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_sort_defaults_when_nothing_stored() {
        let sort = DashboardSort::from_stored(None, None);
        assert_eq!(sort, DashboardSort::default());
        assert_eq!(sort.years, SortDirection::Desc);
        assert_eq!(sort.projects, SortDirection::Asc);
    }

    #[test]
    fn dashboard_sort_reads_stored_values() {
        let sort = DashboardSort::from_stored(Some("asc"), Some("desc"));
        assert_eq!(sort.years, SortDirection::Asc);
        assert_eq!(sort.projects, SortDirection::Desc);
    }

    #[test]
    fn dashboard_sort_unknown_value_falls_back_per_key() {
        let sort = DashboardSort::from_stored(Some("sideways"), Some("desc"));
        assert_eq!(sort.years, SortDirection::Desc);
        assert_eq!(sort.projects, SortDirection::Desc);
    }

    #[test]
    fn sort_direction_round_trips_as_str() {
        for d in [SortDirection::Asc, SortDirection::Desc] {
            assert_eq!(SortDirection::parse(Some(d.as_str())), Some(d));
        }
    }

    #[test]
    fn slug_replaces_slash_and_trims() {
        assert_eq!(ProjectFolder::slug("  Boda García  "), "Boda García");
        assert_eq!(ProjectFolder::slug("a/b/c"), "a-b-c");
    }

    #[test]
    fn folder_name_prefixes_day() {
        assert_eq!(ProjectFolder::folder_name("15", "Boda"), "15_Boda");
    }

    #[test]
    fn date_parts_iso_and_exif() {
        assert_eq!(
            ProjectFolder::date_parts("2024-06-15T10:00:00Z"),
            Some(("2024".into(), "06".into(), "15".into()))
        );
        // EXIF usa ':' como separador de fecha
        assert_eq!(
            ProjectFolder::date_parts("2024:06:15 10:00:00"),
            Some(("2024".into(), "06".into(), "15".into()))
        );
    }

    #[test]
    fn date_parts_rejects_short_or_malformed() {
        assert_eq!(ProjectFolder::date_parts("2024-06"), None);
        assert_eq!(ProjectFolder::date_parts(""), None);
        // 10 chars pero sin tres segmentos
        assert_eq!(ProjectFolder::date_parts("2024/06/15"), None);
    }

    #[test]
    fn path_builds_date_based_layout() {
        let mount = Path::new("/mnt/disk");
        let p = ProjectFolder::path(mount, "2024", "06", "15", "Boda");
        assert_eq!(p, PathBuf::from("/mnt/disk/lumik/2024/06/15_Boda"));
    }

    #[test]
    fn dashboard_orders_newest_first_nulls_last() {
        let mut rows = vec![
            (Some("2024-01-01"), "c1"),
            (None, "c2"),
            (Some("2024-06-01"), "c3"),
            (None, "c4"),
        ];
        rows.sort_by(|a, b| compare_dashboard((a.0, a.1), (b.0, b.1)));
        // session_date DESC primero, NULLs al final
        assert_eq!(rows[0].0, Some("2024-06-01"));
        assert_eq!(rows[1].0, Some("2024-01-01"));
        assert!(rows[2].0.is_none() && rows[3].0.is_none());
    }

    #[test]
    fn dashboard_desempata_por_created_desc() {
        let mut rows = vec![
            (Some("2024-06-01"), "2024-06-01T08:00:00"),
            (Some("2024-06-01"), "2024-06-01T20:00:00"),
        ];
        rows.sort_by(|a, b| compare_dashboard((a.0, a.1), (b.0, b.1)));
        // mismo session_date → created_at más reciente primero
        assert_eq!(rows[0].1, "2024-06-01T20:00:00");
    }
}
