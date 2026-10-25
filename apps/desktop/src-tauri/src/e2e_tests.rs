//! Tests de punta a punta: casos de uso del núcleo con la infraestructura
//! real del desktop (disco, sidecars AppleDouble, SQLite). Viven en el shell
//! porque el núcleo no conoce implementaciones concretas.

use crate::application::use_cases::trash_photo::{RestorePhoto, TrashPhoto};
use crate::db::models::CreatePhoto;
use crate::db::ProjectDatabase;
use crate::infrastructure::finder_tags::AppleDoubleFinderTags;
use crate::infrastructure::fs::StdFileStore;

// ---- Disco y SQLite reales -------------------------------------------------

/// Recorrido completo sobre un directorio temporal, con `StdFileStore`,
/// sidecars AppleDouble reales y una `project.db` real: aprobada + exportado +
/// portada → papelera → restaurar.
#[test]
fn trash_and_restore_on_real_disk_and_db() {

    let mount = std::env::temp_dir().join(format!("lumik_trash_e2e_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&mount);
    let project_rel = "lumik/2024/06/15_Boda";
    let project_dir = mount.join(project_rel);
    let db = ProjectDatabase::create(
        project_dir.join("project.db"), "p1", "Boda", "creator", None, Some("2024-06-15"), "dev", mount.clone(),
    )
    .unwrap();

    let write = |rel: &str| {
        let path = project_dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, rel.as_bytes()).unwrap();
    };
    write("_culled/IMG_0001.cr2");
    write("_culled/IMG_0001.xmp");
    write("_culled/._IMG_0001.cr2");
    write("_exported/IMG_0001.jpg");

    let created = db
        .create_photos_batch(&[CreatePhoto {
            project_id: "p1".into(),
            dng_path: "_culled/IMG_0001.cr2".into(),
            device_uuid: "dev".into(),
            original_camera: None,
            original_format: None,
            capture_date: None,
            width: None,
            height: None,
            file_size_bytes: None,
            iso: None,
            aperture: None,
            shutter_speed: None,
            exposure_compensation: None,
            focal_length: None,
            lens_model: None,
            rotation: 0,
            ..Default::default()
        }])
        .unwrap();
    let id = created[0].id.clone();
    db.update_photo_culled(&id, true, "_culled/IMG_0001.cr2").unwrap();
    db.set_photo_exported(&id, "_exported/IMG_0001.jpg").unwrap();
    db.set_cover_photo(Some(&format!(".thumbs/{}.jpg", id))).unwrap();

    let fs = StdFileStore;
    let tags = AppleDoubleFinderTags;

    TrashPhoto { photos: &db, files: &fs, finder_tags: &tags, project_dir: &project_dir }.execute(&id).unwrap();

    for rel in ["_trash/IMG_0001.cr2", "_trash/IMG_0001.xmp", "_trash/._IMG_0001.cr2", "_trash/_exported/IMG_0001.jpg"] {
        assert!(project_dir.join(rel).exists(), "falta {}", rel);
    }
    for rel in ["_culled/IMG_0001.cr2", "_culled/._IMG_0001.cr2", "_exported/IMG_0001.jpg"] {
        assert!(!project_dir.join(rel).exists(), "sobra {}", rel);
    }
    assert!(db.get_project_photos().unwrap().is_empty());
    let entry = db.get_project_dashboard_entry().unwrap().unwrap();
    assert_eq!(entry.photo_count, 0);
    assert_eq!(entry.cover_photo_path, None);

    RestorePhoto { photos: &db, files: &fs, finder_tags: &tags, project_dir: &project_dir }.execute(&id).unwrap();

    for rel in ["_culled/IMG_0001.cr2", "_culled/IMG_0001.xmp", "_culled/._IMG_0001.cr2", "_exported/IMG_0001.jpg"] {
        assert!(project_dir.join(rel).exists(), "falta {}", rel);
    }
    let photo = db.get_photo(&id).unwrap().unwrap();
    assert!(!photo.deleted);
    assert!(photo.culled);
    assert_eq!(photo.dng_path, "_culled/IMG_0001.cr2");
    assert_eq!(photo.jpg_path.as_deref(), Some("_exported/IMG_0001.jpg"));
    // El contenido viajó intacto: nada se sobrescribió.
    assert_eq!(std::fs::read(project_dir.join("_culled/IMG_0001.cr2")).unwrap(), b"_culled/IMG_0001.cr2");

    drop(db);
    let _ = std::fs::remove_dir_all(&mount);
}
