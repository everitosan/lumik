//! Puente de `lumik-core` hacia la app de tablet (React Native) con uniffi:
//! los casos de uso del núcleo sobre el disco por SAF, el import de la tarjeta,
//! la copia de trabajo de `project.db`, el candado y el nombre del disco.

mod disk;
mod disk_name;
mod histogram;
mod import;
mod lock;
mod mirror;
mod project;
mod projects;

pub use disk::{cull_photo, delivery_mode, plan_delivery_change, rate_photo, rotate_photo, set_delivery_mode, write_sidecar_orientation, DeliveryPlanView, DiskError, DiskFile, DiskFiles, DiskFingerprint};
pub use import::{make_placeholders, start_card_import, CardFile, CardImportListener, CardImportResult, CardSource, ImportSettings, ImportStatus};
pub use lock::{install_identity, lock_view, own_lock_json, set_install_name, InstallInfo, LockView};
pub use histogram::{start_histogram, Histogram, HistogramListener};
pub use mirror::{read_text, write_text, read_fd_string, write_fd_string, missing_files, start_path_to_fd, copy_fd_to_fd, copy_fd_to_path, copy_path_to_fd, fingerprint_fd, start_copy_to_dir, start_fd_to_path, reset_dir, vacuum_into, CopyListener, CopyResult, MirrorError};
pub use project::{create_project_db, open_project, open_project_trash, NewProject, PhotoInfo, ProjectError};

uniffi::setup_scaffolding!();

#[cfg(test)]
mod project_tests {
    use super::*;

    #[test]
    fn histogram_counts_every_pixel() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rojo.jpg");
        image::RgbImage::from_pixel(40, 30, image::Rgb([250, 10, 10])).save(&path).unwrap();

        let h = histogram::histogram_of(path.to_str().unwrap()).unwrap();

        assert_eq!(h.r.iter().sum::<u32>(), 1200);
        // JPEG con pérdida: el rojo queda cerca de 250, no exacto.
        assert!(h.r[240..].iter().sum::<u32>() > 1100);
        assert!(h.g[..30].iter().sum::<u32>() > 1100);
    }
}

/// Con un proyecto real: `LUMIK_PROJECT=/ruta/al/proyecto cargo test -- --ignored`.
/// Trabaja sobre una copia de `project.db`, porque abrirlo aplica migraciones.
#[cfg(test)]
#[test]
#[ignore]
fn open_project_reads_a_real_project() {
    let real = std::path::PathBuf::from(std::env::var("LUMIK_PROJECT").expect("LUMIK_PROJECT"));
    let copy = tempfile::tempdir().unwrap();
    std::fs::copy(real.join("project.db"), copy.path().join("project.db")).unwrap();

    let photos = open_project(copy.path().display().to_string()).unwrap();

    assert!(!photos.is_empty());
    let p = &photos[0];
    eprintln!("{} fotos; la primera: {} {:?} {:?} {:?} {:?} {:?} rot {}", photos.len(), p.file_name, p.camera, p.lens_model, p.shutter_speed, p.aperture, p.iso, p.rotation);
    assert!(p.thumb_path.ends_with(&format!(".thumbs/{}.jpg", p.id)));
}
