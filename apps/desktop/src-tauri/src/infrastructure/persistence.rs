//! Implementaciones de los puertos de repositorio sobre las bases SQLite
//! existentes (`ProjectDatabase`/`GlobalDatabase`). Son adaptadores delgados:
//! delegan en los métodos de `db::queries` y mapean `DbError` → `String`.

use crate::application::ports::PhotoRepository;
use crate::db::models::Photo;
use crate::db::ProjectDatabase;

impl PhotoRepository for ProjectDatabase {
    fn get(&self, id: &str) -> Result<Option<Photo>, String> {
        self.get_photo(id).map_err(|e| e.to_string())
    }

    fn update_culled(&self, id: &str, culled: bool, new_dng_path: &str) -> Result<(), String> {
        self.update_photo_culled(id, culled, new_dng_path)
            .map_err(|e| e.to_string())
    }
}
