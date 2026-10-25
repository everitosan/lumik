//! Pares RAW+JPEG (spec `photo-pairs`): un RAW y un JPEG de la misma toma se
//! registran como una sola foto. La toma se reconoce por el nombre de cámara
//! sin extensión y la fecha de captura al segundo. Reglas puras: sin fs ni BD.

use std::collections::HashMap;
use std::path::Path;

/// Extensiones RAW admitidas por el import (mismas que `photo-import`).
const RAW_EXTENSIONS: &[&str] = &["cr2", "cr3", "nef", "arw", "raf", "orf", "rw2", "dng"];
const JPEG_EXTENSIONS: &[&str] = &["jpg", "jpeg"];

/// Tipo de archivo de una foto, a efectos de emparejar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileKind {
    Raw,
    Jpeg,
    /// TIFF u otro formato: nunca forma par.
    Other,
}

impl FileKind {
    pub fn of(path: &Path) -> FileKind {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if RAW_EXTENSIONS.contains(&ext.as_str()) {
            FileKind::Raw
        } else if JPEG_EXTENSIONS.contains(&ext.as_str()) {
            FileKind::Jpeg
        } else {
            FileKind::Other
        }
    }

    /// Tipo que completa un par con este (`Raw` ↔ `Jpeg`).
    pub fn partner(self) -> Option<FileKind> {
        match self {
            FileKind::Raw => Some(FileKind::Jpeg),
            FileKind::Jpeg => Some(FileKind::Raw),
            FileKind::Other => None,
        }
    }
}

/// Identidad de una toma: nombre de cámara sin extensión (en minúsculas) y
/// fecha de captura al segundo. Sin fecha no hay clave: el archivo no se empareja.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShotKey {
    pub stem: String,
    pub captured: String,
}

impl ShotKey {
    /// `stem` es el nombre de cámara sin extensión (`DSCF1234`); `capture_date`
    /// la fecha EXIF (`2026:10:20 18:04:11`, con o sin fracción ni zona).
    pub fn new(stem: &str, capture_date: Option<&str>) -> Option<ShotKey> {
        let captured = capture_to_second(capture_date?)?;
        let stem = stem.trim().to_lowercase();
        (!stem.is_empty()).then_some(ShotKey { stem, captured })
    }
}

/// Recorta una fecha EXIF/ISO a `AAAA-MM-DD HH:MM:SS` (los primeros 19
/// caracteres con separadores normalizados). `None` si es demasiado corta.
fn capture_to_second(value: &str) -> Option<String> {
    let v = value.trim();
    if v.len() < 19 || !v.is_char_boundary(19) {
        return None;
    }
    let head = &v[..19];
    let normalized: String = head
        .chars()
        .enumerate()
        .map(|(i, c)| match (i, c) {
            (4 | 7, ':') => '-',
            (10, 'T') => ' ',
            _ => c,
        })
        .collect();
    Some(normalized)
}

/// Nombre de cámara sin extensión de una ruta (`/a/DSCF1234.RAF` → `DSCF1234`).
pub fn stem_of(path: &Path) -> String {
    path.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string()
}

/// Un archivo a agrupar: su clave (si tiene) y su tipo.
#[derive(Debug, Clone)]
pub struct ShotFile<T> {
    pub item: T,
    pub key: Option<ShotKey>,
    pub kind: FileKind,
}

/// Una toma: un par RAW+JPEG o un archivo suelto.
#[derive(Debug, Clone, PartialEq)]
pub enum Shot<T> {
    Pair { raw: T, jpg: T },
    Single(T),
}

/// Agrupa archivos en tomas. Un grupo con la misma clave forma par solo si
/// tiene exactamente un RAW y un JPEG; cualquier otra combinación queda en
/// sueltos. Conserva el orden de aparición (el de la primera foto de cada toma).
pub fn group_shots<T>(files: Vec<ShotFile<T>>) -> Vec<Shot<T>> {
    // Cuántos archivos de cada tipo hay por clave.
    let mut counts: HashMap<ShotKey, (usize, usize, usize)> = HashMap::new();
    for f in &files {
        if let Some(key) = &f.key {
            let c = counts.entry(key.clone()).or_default();
            match f.kind {
                FileKind::Raw => c.0 += 1,
                FileKind::Jpeg => c.1 += 1,
                FileKind::Other => c.2 += 1,
            }
        }
    }
    let is_pair_key = |key: &ShotKey| counts.get(key) == Some(&(1, 1, 0));

    // Posición de cada toma en la salida y, para los pares, la mitad pendiente.
    let mut shots: Vec<Option<Shot<T>>> = Vec::new();
    let mut open_pairs: HashMap<ShotKey, (usize, FileKind, T)> = HashMap::new();
    for f in files {
        match f.key {
            Some(key) if is_pair_key(&key) => match open_pairs.remove(&key) {
                Some((slot, first_kind, first)) => {
                    let (raw, jpg) = if first_kind == FileKind::Raw { (first, f.item) } else { (f.item, first) };
                    shots[slot] = Some(Shot::Pair { raw, jpg });
                }
                None => {
                    shots.push(None);
                    open_pairs.insert(key, (shots.len() - 1, f.kind, f.item));
                }
            },
            _ => shots.push(Some(Shot::Single(f.item))),
        }
    }
    shots.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &'static str, date: Option<&str>) -> ShotFile<&'static str> {
        let path = Path::new(name);
        ShotFile { item: name, key: ShotKey::new(&stem_of(path), date), kind: FileKind::of(path) }
    }

    const T1: Option<&str> = Some("2026:10:20 18:04:11");

    #[test]
    fn raw_and_jpg_of_the_same_shot_form_a_pair() {
        let shots = group_shots(vec![file("DSCF1234.RAF", T1), file("DSCF1234.JPG", T1)]);
        assert_eq!(shots, vec![Shot::Pair { raw: "DSCF1234.RAF", jpg: "DSCF1234.JPG" }]);
    }

    #[test]
    fn pairs_across_folders_and_in_any_order() {
        // Doble ranura: el JPG llega antes y de otra carpeta.
        let shots = group_shots(vec![file("SLOT2/DSCF1234.JPG", T1), file("SLOT1/dscf1234.raf", T1)]);
        assert_eq!(shots, vec![Shot::Pair { raw: "SLOT1/dscf1234.raf", jpg: "SLOT2/DSCF1234.JPG" }]);
    }

    #[test]
    fn same_name_from_another_session_does_not_pair() {
        let shots = group_shots(vec![
            file("DSCF0001.RAF", Some("2025:03:01 10:00:00")),
            file("DSCF0001.JPG", Some("2026:03:01 10:00:00")),
        ]);
        assert_eq!(shots, vec![Shot::Single("DSCF0001.RAF"), Shot::Single("DSCF0001.JPG")]);
    }

    #[test]
    fn files_without_partner_stay_single() {
        let shots = group_shots(vec![file("DSCF0007.RAF", T1), file("DSCF0008.JPG", T1)]);
        assert_eq!(shots, vec![Shot::Single("DSCF0007.RAF"), Shot::Single("DSCF0008.JPG")]);
    }

    #[test]
    fn more_than_two_files_with_the_same_key_do_not_pair() {
        let shots = group_shots(vec![
            file("A/IMG_1.CR2", T1),
            file("B/IMG_1.CR2", T1),
            file("A/IMG_1.JPG", T1),
        ]);
        assert_eq!(shots.len(), 3);
        assert!(shots.iter().all(|s| matches!(s, Shot::Single(_))));
    }

    #[test]
    fn tiff_and_missing_dates_never_pair() {
        let shots = group_shots(vec![
            file("SCAN.TIF", T1),
            file("SCAN.JPG", T1),
            file("IMG_2.CR2", None),
            file("IMG_2.JPG", None),
        ]);
        assert_eq!(shots.len(), 4);
    }

    #[test]
    fn keeps_order_of_first_appearance() {
        let shots = group_shots(vec![
            file("B.RAF", T1),
            file("A.RAF", T1),
            file("B.JPG", T1),
            file("A.JPG", T1),
        ]);
        assert_eq!(
            shots,
            vec![Shot::Pair { raw: "B.RAF", jpg: "B.JPG" }, Shot::Pair { raw: "A.RAF", jpg: "A.JPG" }]
        );
    }

    #[test]
    fn capture_date_is_compared_to_the_second() {
        assert_eq!(
            ShotKey::new("x", Some("2026:10:20 18:04:11.25+02:00")),
            ShotKey::new("X", Some("2026-10-20T18:04:11"))
        );
        assert_eq!(ShotKey::new("x", Some("2026:10")), None);
    }
}
