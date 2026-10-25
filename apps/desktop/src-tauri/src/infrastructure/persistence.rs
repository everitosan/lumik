//! Implementaciones de los puertos de repositorio sobre las bases SQLite
//! existentes (`ProjectDatabase`/`GlobalDatabase`). Son adaptadores delgados:
//! delegan en los métodos de `db::queries` y mapean `DbError` → `String`.

use crate::application::ports::{CullUpdate, PhotoRepository};
use crate::db::models::{CreatePhoto, Photo};
use crate::db::ProjectDatabase;
use std::collections::HashSet;

impl PhotoRepository for ProjectDatabase {
    fn get(&self, id: &str) -> Result<Option<Photo>, String> {
        self.get_photo(id).map_err(|e| e.to_string())
    }

    fn update_rotation(&self, id: &str, rotation: i32) -> Result<(), String> {
        self.update_photo_rotation(id, rotation).map_err(|e| e.to_string())
    }

    fn update_rating(
        &self,
        id: &str,
        stars: i32,
        color_label: Option<&str>,
        tags: Option<&str>,
    ) -> Result<(), String> {
        self.update_photo_rating(id, stars, color_label, tags)
            .map_err(|e| e.to_string())
    }

    fn update_culled(&self, id: &str, culled: bool, new_dng_path: &str) -> Result<(), String> {
        self.update_photo_culled(id, culled, new_dng_path)
            .map_err(|e| e.to_string())
    }

    fn update_cull(&self, id: &str, update: &CullUpdate) -> Result<(), String> {
        self.update_photo_cull(id, update).map_err(|e| e.to_string())
    }

    fn update_dng_path(&self, id: &str, new_dng_path: &str) -> Result<(), String> {
        self.update_photo_dng_path(id, new_dng_path).map_err(|e| e.to_string())
    }

    fn set_exported(&self, id: &str, jpg_path: &str) -> Result<(), String> {
        self.set_photo_exported(id, jpg_path).map_err(|e| e.to_string())
    }

    fn set_deleted(
        &self,
        id: &str,
        deleted: bool,
        dng_path: &str,
        jpg_path: Option<&str>,
        camera_jpg_path: Option<&str>,
    ) -> Result<(), String> {
        self.set_photo_deleted(id, deleted, dng_path, jpg_path, camera_jpg_path)
            .map_err(|e| e.to_string())
    }

    fn create_batch(&self, photos: &[CreatePhoto]) -> Result<Vec<Photo>, String> {
        self.create_photos_batch(photos).map_err(|e| e.to_string())
    }

    fn registered_dng_paths(&self, dng_paths: &[String]) -> Result<HashSet<String>, String> {
        self.get_registered_dng_paths(dng_paths).map_err(|e| e.to_string())
    }

    fn approved_photos(&self) -> Result<Vec<Photo>, String> {
        self.get_approved_photos().map_err(|e| e.to_string())
    }

    fn loose_photos(&self) -> Result<Vec<Photo>, String> {
        self.get_loose_photos().map_err(|e| e.to_string())
    }

    fn join_partner(&self, id: &str, rel_path: &str, raw_format: Option<&str>) -> Result<(), String> {
        self.join_photo_partner(id, rel_path, raw_format).map_err(|e| e.to_string())
    }
}
