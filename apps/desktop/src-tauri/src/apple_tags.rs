//! Escritura de Finder tags de macOS/iPadOS en discos sin soporte de extended
//! attributes (exFAT/FAT), mediante sidecars AppleDouble `._<nombre>`.
//!
//! macOS/iPadOS guardan las etiquetas en el xattr
//! `com.apple.metadata:_kMDItemUserTags`, un binary plist con un array de strings
//! `"Nombre\n<índice_de_color>"`. En un filesystem sin xattrs (exFAT) ese atributo
//! se materializa en un archivo hermano `._<nombre>` con formato AppleDouble. Este
//! módulo genera ese archivo byte a byte; el formato se verificó decodificando
//! archivos reales escritos por un iPad sobre el mismo disco.
//!
//! Índices de color (confirmados 1/4/6/7 con datos del iPad, resto estándar Apple):
//! 0=sin color · 1=gris · 2=verde · 3=morado · 4=azul · 5=amarillo · 6=rojo · 7=naranja.

use std::io;
use std::path::{Path, PathBuf};

const TAGS_XATTR: &str = "com.apple.metadata:_kMDItemUserTags";

/// Resource fork "en blanco" que macOS/iPadOS incluye como entrada id 2 de todo
/// AppleDouble. Extraído tal cual de un `._` real.
const RESFORK_BLANK: [u8; 286] = [
    0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x1e, 0x54, 0x68, 0x69, 0x73, 0x20, 0x72, 0x65, 0x73,
    0x6f, 0x75, 0x72, 0x63, 0x65, 0x20, 0x66, 0x6f, 0x72, 0x6b, 0x20, 0x69,
    0x6e, 0x74, 0x65, 0x6e, 0x74, 0x69, 0x6f, 0x6e, 0x61, 0x6c, 0x6c, 0x79,
    0x20, 0x6c, 0x65, 0x66, 0x74, 0x20, 0x62, 0x6c, 0x61, 0x6e, 0x6b, 0x20,
    0x20, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1e, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x1c, 0x00, 0x1e, 0xff, 0xff,
];

/// Mapea un color label de Lumik (1..5) al par (nombre, índice de color macOS).
/// Los nombres son los presets en español para que se fundan con las etiquetas
/// nativas del iPad; el índice es lo que determina el color mostrado.
pub fn lumik_color_to_macos(id: u8) -> Option<(&'static str, u8)> {
    match id {
        1 => Some(("Rojo", 6)),
        2 => Some(("Amarillo", 5)),
        3 => Some(("Verde", 2)),
        4 => Some(("Azul", 4)),
        5 => Some(("Morado", 3)),
        _ => None,
    }
}

/// Convierte un `color_label` de la BD (`"1,3,5"`) en la lista de tags macOS.
pub fn colors_from_label(label: &str) -> Vec<(&'static str, u8)> {
    label
        .split(',')
        .filter_map(|s| s.trim().parse::<u8>().ok())
        .filter_map(lumik_color_to_macos)
        .collect()
}

/// Ruta del sidecar `._<nombre>` para un archivo dado.
fn sidecar_path(target: &Path) -> Option<PathBuf> {
    let name = target.file_name()?;
    let mut prefixed = std::ffi::OsString::from("._");
    prefixed.push(name);
    Some(target.with_file_name(prefixed))
}

/// Escribe (o sobrescribe) el sidecar de Finder tags de `target` con `colors`.
///
/// Nota: sobrescribe cualquier `._` previo con solo el xattr de tags — asume el
/// flujo unidireccional Lumik→iPad. Los RAW recién importados no traen sidecar.
pub fn write_color_tags(target: &Path, colors: &[(&str, u8)]) -> io::Result<()> {
    let tags: Vec<String> = colors
        .iter()
        .map(|(name, idx)| format!("{}\n{}", name, idx))
        .collect();
    let plist = build_tags_plist(&tags);
    let ad = build_appledouble(TAGS_XATTR, &plist);
    let sidecar = sidecar_path(target)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "ruta sin nombre de archivo"))?;
    std::fs::write(&sidecar, &ad)
}

/// Borra el sidecar `._<nombre>` de `target` si existe (al limpiar el color).
pub fn remove_tags_sidecar(target: &Path) -> io::Result<()> {
    let Some(sidecar) = sidecar_path(target) else {
        return Ok(());
    };
    match std::fs::remove_file(&sidecar) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Mueve el sidecar de `old` a `new` si existe (cuando el archivo se reubica,
/// p.ej. al cullar). No falla si no hay sidecar.
pub fn move_sidecar(old: &Path, new: &Path) -> io::Result<()> {
    let (Some(src), Some(dst)) = (sidecar_path(old), sidecar_path(new)) else {
        return Ok(());
    };
    match std::fs::rename(&src, &dst) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// ¿Hay un índice de Spotlight en la raíz del volumen? Se usa para evitar trabajo
/// si ya fue invalidado (el índice no reaparece hasta que un iPad/Mac lo reconstruye).
pub fn spotlight_index_present(volume_root: &Path) -> bool {
    volume_root.join(".Spotlight-V100").exists()
}

/// Invalida el índice de Spotlight del volumen borrando `.Spotlight-V100` y
/// `.fseventsd` en la raíz del disco.
///
/// Necesario para el flujo iPad/Mac: los tags escritos en los sidecars solo se
/// pintan como punto de color, pero el **filtro por etiquetas** del sidebar de
/// Archivos/Finder es una búsqueda de Spotlight contra su índice. Como los cambios
/// hechos desde Linux no generan eventos de `fseventsd`, el sistema nunca reindexa
/// esos archivos. Al borrar el índice, iPadOS/macOS lo reconstruye al reconectar el
/// disco, leyendo los tags de los sidecars — y entonces el filtro los reconoce.
///
/// Ambos directorios son metadata regenerable por el SO; es seguro borrarlos.
pub fn invalidate_spotlight_index(volume_root: &Path) -> io::Result<()> {
    for dir in [".Spotlight-V100", ".fseventsd"] {
        match std::fs::remove_dir_all(volume_root.join(dir)) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[inline]
fn align4(x: usize) -> usize {
    (x + 3) & !3
}

/// Serializa un array de strings ASCII cortas como binary plist (`bplist00`),
/// idéntico byte a byte al que produce macOS/iPadOS para `_kMDItemUserTags`.
///
/// Solo cubre el caso acotado de este módulo (≤14 tags, cada uno ASCII y <15 bytes),
/// donde todos los objetos usan marcadores compactos y offsets de 1 byte.
fn build_tags_plist(tags: &[String]) -> Vec<u8> {
    debug_assert!(tags.len() < 15, "formato compacto: máx 14 tags");
    let num_objects = 1 + tags.len();
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"bplist00");

    let mut offsets: Vec<usize> = Vec::with_capacity(num_objects);
    // obj 0: array (objeto raíz). Referencias a los objetos 1..=n (ref size = 1).
    offsets.push(out.len());
    out.push(0xA0 | tags.len() as u8);
    for i in 0..tags.len() {
        out.push((i + 1) as u8);
    }
    // obj 1..=n: strings ASCII (marcador 0x5X, X = longitud < 15).
    for t in tags {
        let b = t.as_bytes();
        debug_assert!(b.is_ascii() && b.len() < 15, "tag debe ser ASCII y <15 bytes");
        offsets.push(out.len());
        out.push(0x50 | b.len() as u8);
        out.extend_from_slice(b);
    }

    let offset_table_offset = out.len();
    debug_assert!(offset_table_offset < 256, "offsets de 1 byte");
    for &off in &offsets {
        out.push(off as u8);
    }

    // trailer (32 bytes)
    out.extend_from_slice(&[0u8; 5]); // sin usar
    out.push(0); // sort version
    out.push(1); // offset_int_size
    out.push(1); // object_ref_size
    out.extend_from_slice(&(num_objects as u64).to_be_bytes());
    out.extend_from_slice(&0u64.to_be_bytes()); // objeto raíz = 0
    out.extend_from_slice(&(offset_table_offset as u64).to_be_bytes());
    out
}

/// Construye un AppleDouble con un único xattr, replicando la estructura que
/// escribe iPadOS: header + entrada FinderInfo (id 9, con bloque ATTR) + entrada
/// ResourceFork (id 2, en blanco).
fn build_appledouble(xattr_name: &str, xattr_value: &[u8]) -> Vec<u8> {
    const FI_OFFSET: usize = 50; // header(24) + num_entries(2) + 2 entradas(24)
    let attr_off = align4(FI_OFFSET + 32); // ATTR tras los 32 bytes de FinderInfo -> 84
    let name_bytes = xattr_name.as_bytes();
    let namelen = name_bytes.len() + 1; // incluye el null final
    let entry_size = align4(10 + 1 + namelen); // entrada de atributo alineada a 4
    let data_start = align4(attr_off + 36 + entry_size); // 36 = tamaño del header ATTR
    let data_length = xattr_value.len();
    let end_data = data_start + data_length;
    let rf_offset = align4(end_data);
    let fi_len = rf_offset - FI_OFFSET;
    let total_size = rf_offset;

    // --- región de la entrada FinderInfo (comienza en FI_OFFSET) ---
    let mut region: Vec<u8> = Vec::new();
    region.extend_from_slice(&[0u8; 32]); // FinderInfo (sin usar)
    while FI_OFFSET + region.len() < attr_off {
        region.push(0);
    }
    // header ATTR (36 bytes)
    region.extend_from_slice(b"ATTR");
    region.extend_from_slice(&0u32.to_be_bytes()); // debug_tag
    region.extend_from_slice(&(total_size as u32).to_be_bytes());
    region.extend_from_slice(&(data_start as u32).to_be_bytes());
    region.extend_from_slice(&(data_length as u32).to_be_bytes());
    region.extend_from_slice(&[0u8; 12]); // reserved
    region.extend_from_slice(&0u16.to_be_bytes()); // flags
    region.extend_from_slice(&1u16.to_be_bytes()); // num_attrs
    // entrada de atributo
    region.extend_from_slice(&(data_start as u32).to_be_bytes()); // offset del valor (absoluto)
    region.extend_from_slice(&(data_length as u32).to_be_bytes());
    region.extend_from_slice(&0u16.to_be_bytes()); // flags
    region.push(namelen as u8);
    region.extend_from_slice(name_bytes);
    region.push(0); // null
    while FI_OFFSET + region.len() < data_start {
        region.push(0);
    }
    // datos del valor + relleno hasta el resource fork
    region.extend_from_slice(xattr_value);
    while FI_OFFSET + region.len() < rf_offset {
        region.push(0);
    }

    // --- ensamblado final ---
    let mut out: Vec<u8> = Vec::with_capacity(FI_OFFSET + region.len() + RESFORK_BLANK.len());
    out.extend_from_slice(&[0x00, 0x05, 0x16, 0x07]); // magic AppleDouble
    out.extend_from_slice(&[0x00, 0x02, 0x00, 0x00]); // versión 2
    out.extend_from_slice(b"Mac OS X");
    out.extend_from_slice(&[b' '; 8]); // filler (16 bytes en total)
    out.extend_from_slice(&2u16.to_be_bytes()); // num_entries
    out.extend_from_slice(&9u32.to_be_bytes()); // entrada FinderInfo (id 9)
    out.extend_from_slice(&(FI_OFFSET as u32).to_be_bytes());
    out.extend_from_slice(&(fi_len as u32).to_be_bytes());
    out.extend_from_slice(&2u32.to_be_bytes()); // entrada ResourceFork (id 2)
    out.extend_from_slice(&(rf_offset as u32).to_be_bytes());
    out.extend_from_slice(&(RESFORK_BLANK.len() as u32).to_be_bytes());
    debug_assert_eq!(out.len(), FI_OFFSET);
    out.extend_from_slice(&region);
    out.extend_from_slice(&RESFORK_BLANK);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Bytes reales del xattr _kMDItemUserTags escritos por un iPad (golden).
    const IPAD_RED: &[u8] = &[
        98, 112, 108, 105, 115, 116, 48, 48, 161, 1, 85, 82, 101, 100, 10, 54, 8, 10, 0, 0, 0, 0,
        0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16,
    ];
    const IPAD_MULTI: &[u8] = &[
        98, 112, 108, 105, 115, 116, 48, 48, 162, 1, 2, 85, 82, 101, 100, 10, 54, 88, 80, 104, 111,
        116, 111, 115, 10, 52, 8, 11, 17, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 26,
    ];

    #[test]
    fn bplist_single_matches_ipad() {
        assert_eq!(build_tags_plist(&["Red\n6".to_string()]), IPAD_RED);
    }

    #[test]
    fn bplist_multi_matches_ipad() {
        assert_eq!(
            build_tags_plist(&["Red\n6".to_string(), "Photos\n4".to_string()]),
            IPAD_MULTI
        );
    }

    #[test]
    fn color_map_matches_lumik_ids() {
        assert_eq!(lumik_color_to_macos(1), Some(("Rojo", 6)));
        assert_eq!(lumik_color_to_macos(5), Some(("Morado", 3)));
        assert_eq!(lumik_color_to_macos(9), None);
        assert_eq!(
            colors_from_label("1,3,5"),
            vec![("Rojo", 6), ("Verde", 2), ("Morado", 3)]
        );
    }

    /// Parser mínimo del AppleDouble: extrae el valor del primer xattr. Sirve para
    /// verificar que lo que escribimos se puede volver a leer correctamente.
    fn parse_first_xattr(data: &[u8]) -> (String, Vec<u8>) {
        let be32 = |o: usize| u32::from_be_bytes(data[o..o + 4].try_into().unwrap()) as usize;
        let be16 = |o: usize| u16::from_be_bytes(data[o..o + 2].try_into().unwrap()) as usize;
        assert_eq!(&data[0..4], &[0x00, 0x05, 0x16, 0x07]);
        let num_entries = be16(24);
        let mut fi_off = 0;
        for i in 0..num_entries {
            let base = 26 + i * 12;
            if be32(base) == 9 {
                fi_off = be32(base + 4);
            }
        }
        // localizar "ATTR" a partir del inicio de la región FinderInfo
        let attr = data[fi_off..]
            .windows(4)
            .position(|w| w == b"ATTR")
            .map(|p| fi_off + p)
            .unwrap();
        let num_attrs = be16(attr + 34);
        assert!(num_attrs >= 1);
        let e = attr + 36; // primera entrada
        let voff = be32(e);
        let vlen = be32(e + 4);
        let nl = data[e + 10] as usize;
        let name = String::from_utf8(data[e + 11..e + 11 + nl - 1].to_vec()).unwrap();
        (name, data[voff..voff + vlen].to_vec())
    }

    #[test]
    fn appledouble_roundtrips() {
        let colors = [("Rojo", 6u8), ("Azul", 4u8)];
        let tags: Vec<String> = colors
            .iter()
            .map(|(n, i)| format!("{}\n{}", n, i))
            .collect();
        let plist = build_tags_plist(&tags);
        let ad = build_appledouble(TAGS_XATTR, &plist);
        let (name, value) = parse_first_xattr(&ad);
        assert_eq!(name, TAGS_XATTR);
        assert_eq!(value, plist);
    }
}
