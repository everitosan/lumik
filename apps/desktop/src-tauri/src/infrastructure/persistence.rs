//! Implementaciones de los puertos de repositorio sobre las bases SQLite
//! existentes (`ProjectDatabase`/`GlobalDatabase`). Son adaptadores delgados:
//! delegan en los métodos de `db::queries` y mapean `DbError` → `String`.

use crate::application::ports::PhotoRepository;
use crate::db::models::{CreatePhoto, Photo};
use crate::db::ProjectDatabase;

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

    fn create_batch(&self, photos: &[CreatePhoto]) -> Result<Vec<Photo>, String> {
        self.create_photos_batch(photos).map_err(|e| e.to_string())
    }
}
