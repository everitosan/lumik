//! Nombre del disco guardado en el disco (`lumik/.disk.json`, spec
//! `device-detection`): lectura para nombrar los discos sin etiqueta y
//! escritura atómica cuando hay que copiar la etiqueta.

use crate::domain::disk_name::{DiskName, DISK_NAME_FILE};
use std::path::Path;

pub struct FsDiskNameStore;

impl FsDiskNameStore {
    /// El nombre guardado en el disco montado en `mount`. Sin archivo o con uno
    /// ilegible, `None`.
    pub fn read(&self, mount: &Path) -> Option<DiskName> {
        let text = std::fs::read_to_string(mount.join(DISK_NAME_FILE)).ok()?;
        DiskName::from_json(&text)
    }

    /// Escribe el archivo con temporal y `rename`. La carpeta `lumik/` debe
    /// existir: un disco sin ella no se nombra.
    pub fn write(&self, mount: &Path, name: &DiskName) -> Result<(), String> {
        let path = mount.join(DISK_NAME_FILE);
        let tmp = mount.join(format!("{DISK_NAME_FILE}.tmp"));
        std::fs::write(&tmp, name.to_json())
            .map_err(|e| format!("no se pudo escribir el nombre del disco: {e}"))?;
        std::fs::rename(&tmp, &path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("no se pudo escribir el nombre del disco: {e}")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::disk_name::DiskNameSource;

    #[test]
    fn write_read_and_damaged() {
        let mount = std::env::temp_dir().join(format!("lumik_disk_name_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&mount);
        std::fs::create_dir_all(mount.join("lumik")).unwrap();
        let store = FsDiskNameStore;

        assert_eq!(store.read(&mount), None);
        let name = DiskName::new("SEAGATE", DiskNameSource::Label, "2026-10-30T13:34:40Z");
        store.write(&mount, &name).unwrap();
        assert_eq!(store.read(&mount), Some(name));
        assert!(!mount.join(format!("{DISK_NAME_FILE}.tmp")).exists(), "no queda el temporal");

        std::fs::write(mount.join(DISK_NAME_FILE), "{roto").unwrap();
        assert_eq!(store.read(&mount), None, "un archivo dañado cuenta como ausente");

        let _ = std::fs::remove_dir_all(&mount);
    }

    #[test]
    fn write_fails_without_lumik_folder() {
        let mount = std::env::temp_dir().join(format!("lumik_disk_name_nofolder_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&mount);
        std::fs::create_dir_all(&mount).unwrap();
        let name = DiskName::new("SEAGATE", DiskNameSource::Label, "2026-10-30T13:34:40Z");
        assert!(FsDiskNameStore.write(&mount, &name).is_err());
        assert!(!mount.join("lumik").exists(), "no crea la carpeta lumik/");
        let _ = std::fs::remove_dir_all(&mount);
    }
}
