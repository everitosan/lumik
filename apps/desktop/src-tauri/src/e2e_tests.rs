//! Tests de punta a punta: casos de uso del núcleo con la infraestructura
//! real del desktop (disco, sidecars AppleDouble, SQLite). Viven en el shell
//! porque el núcleo no conoce implementaciones concretas.

use crate::application::use_cases::trash_photo::{PurgePhoto, RestorePhoto, TrashPhoto};
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

/// Papelera → restaurar con otro nombre (colisión) → papelera → eliminar
/// definitivamente, con disco y SQLite reales: no queda ningún archivo ni registro.
#[test]
fn restore_as_and_purge_on_real_disk_and_db() {
    let mount = std::env::temp_dir().join(format!("lumik_purge_e2e_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&mount);
    let project_dir = mount.join("lumik/2024/06/15_Boda");
    let db = ProjectDatabase::create(
        project_dir.join("project.db"), "p1", "Boda", "creator", None, Some("2024-06-15"), "dev", mount.clone(),
    )
    .unwrap();
    let write = |rel: &str| {
        let path = project_dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, rel.as_bytes()).unwrap();
    };
    write("_media/IMG_0001.cr2");
    write("_media/IMG_0001.xmp");
    write("_exported/IMG_0001.jpg");

    let created = db
        .create_photos_batch(&[CreatePhoto {
            project_id: "p1".into(),
            dng_path: "_media/IMG_0001.cr2".into(),
            device_uuid: "dev".into(),
            ..Default::default()
        }])
        .unwrap();
    let id = created[0].id.clone();
    db.set_photo_exported(&id, "_exported/IMG_0001.jpg").unwrap();
    write(&format!(".thumbs/{}.jpg", id));
    write(&format!(".previews/{}.jpg", id));

    let fs = StdFileStore;
    let tags = AppleDoubleFinderTags;
    let restore = RestorePhoto { photos: &db, files: &fs, finder_tags: &tags, project_dir: &project_dir };
    TrashPhoto { photos: &db, files: &fs, finder_tags: &tags, project_dir: &project_dir }.execute(&id).unwrap();

    // Otro archivo ocupa el lugar: la restauración pide otro nombre.
    write("_media/IMG_0001.cr2");
    assert!(restore.execute(&id).unwrap_err().starts_with("restore_collision:"));
    restore.execute_as(&id, Some("IMG_0001_b")).unwrap();
    for rel in ["_media/IMG_0001_b.cr2", "_media/IMG_0001_b.xmp", "_exported/IMG_0001_b.jpg", "_media/IMG_0001.cr2"] {
        assert!(project_dir.join(rel).exists(), "falta {}", rel);
    }
    let photo = db.get_photo(&id).unwrap().unwrap();
    assert_eq!(photo.dng_path, "_media/IMG_0001_b.cr2");
    assert_eq!(photo.jpg_path.as_deref(), Some("_exported/IMG_0001_b.jpg"));

    // Fuera de la papelera no se puede eliminar.
    let purge = PurgePhoto { photos: &db, files: &fs, project_dir: &project_dir };
    assert!(purge.execute(&id).is_err());

    TrashPhoto { photos: &db, files: &fs, finder_tags: &tags, project_dir: &project_dir }.execute(&id).unwrap();
    purge.execute(&id).unwrap();

    assert!(db.get_photo(&id).unwrap().is_none());
    assert!(db.get_project_trashed_photos().unwrap().is_empty());
    for rel in [
        "_trash/IMG_0001_b.cr2".to_string(),
        "_trash/IMG_0001_b.xmp".to_string(),
        "_trash/_exported/IMG_0001_b.jpg".to_string(),
        format!(".thumbs/{}.jpg", id),
        format!(".previews/{}.jpg", id),
    ] {
        assert!(!project_dir.join(&rel).exists(), "sobra {}", rel);
    }
    // El archivo que ocupaba el lugar no es de esta foto: sigue ahí.
    assert!(project_dir.join("_media/IMG_0001.cr2").exists());
    drop(db);
    let _ = std::fs::remove_dir_all(&mount);
}

/// Lotes (photo-multi-select) con disco y SQLite reales: mandar 3 fotos a la
/// papelera cuando una choca, restaurar el lote con una colisión y eliminar el resto.
#[test]
fn batches_on_real_disk_and_db() {
    use crate::commands::{purge_photos_in, restore_photos_in, trash_photos_in};

    let mount = std::env::temp_dir().join(format!("lumik_batch_e2e_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&mount);
    let project_dir = mount.join("lumik/2024/06/15_Boda");
    let db = ProjectDatabase::create(
        project_dir.join("project.db"), "p1", "Boda", "creator", None, Some("2024-06-15"), "dev", mount.clone(),
    )
    .unwrap();
    let write = |rel: &str| {
        let path = project_dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, rel.as_bytes()).unwrap();
    };
    let names = ["A.cr2", "B.cr2", "C.cr2"];
    for n in names {
        write(&format!("_media/{n}"));
    }
    let ids: Vec<String> = db
        .create_photos_batch(
            &names
                .iter()
                .map(|n| CreatePhoto { project_id: "p1".into(), dng_path: format!("_media/{n}"), device_uuid: "dev".into(), ..Default::default() })
                .collect::<Vec<_>>(),
        )
        .unwrap()
        .into_iter()
        .map(|p| p.id)
        .collect();
    // B ya tiene un archivo con su nombre en `_trash/`: no se puede mover.
    write("_trash/B.cr2");

    let fs = StdFileStore;
    let tags = AppleDoubleFinderTags;

    let report = trash_photos_in(&db, &fs, &tags, &ids);
    assert_eq!(report.done, vec![ids[0].clone(), ids[2].clone()]);
    assert_eq!(report.failed.len(), 1);
    assert_eq!(report.failed[0].name, "B.cr2");
    assert!(project_dir.join("_media/B.cr2").exists(), "la que falló sigue fuera de la papelera");
    assert!(!db.get_photo(&ids[1]).unwrap().unwrap().deleted);

    // Al restaurar, A choca con otro archivo y se queda; C vuelve.
    write("_media/A.cr2");
    let trashed = vec![ids[0].clone(), ids[2].clone()];
    let report = restore_photos_in(&db, &fs, &tags, &trashed);
    assert_eq!(report.done, vec![ids[2].clone()]);
    assert_eq!(report.failed[0].reason, "collision");
    assert!(db.get_photo(&ids[0]).unwrap().unwrap().deleted);
    assert!(project_dir.join("_media/C.cr2").exists());

    // Eliminar el resto: A se borra; C ya no está en la papelera y se rechaza.
    let report = purge_photos_in(&db, &fs, &trashed);
    assert_eq!(report.done, vec![ids[0].clone()]);
    assert_eq!(report.failed.len(), 1);
    assert!(db.get_photo(&ids[0]).unwrap().is_none());
    assert!(!project_dir.join("_trash/A.cr2").exists());
    assert!(project_dir.join("_media/C.cr2").exists());

    drop(db);
    let _ = std::fs::remove_dir_all(&mount);
}
