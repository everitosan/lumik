//! Caso de uso: al abrir un proyecto, asegurar que existe en el catálogo del
//! server. Si no existe, lo publica (proyecto + fotos). Idempotente y tolerante:
//! degrada silenciosamente sin sesión o sin red. Ver docs/backup-plan.md Fase 5.

use crate::application::ports::{CatalogClient, CatalogPhoto, CatalogProject};

/// Resultado de la sincronización al abrir el proyecto.
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOutcome {
    /// No hay sesión de respaldo: modo solo-local.
    SkippedNoSession,
    /// Sin conexión con el server: se mantiene el estado local.
    Offline,
    /// El proyecto ya estaba en el catálogo.
    AlreadyExists,
    /// El proyecto (y sus fotos) se publicaron ahora.
    Published,
}

/// Mensaje que emite la infraestructura HTTP cuando no hay conexión.
fn is_offline(err: &str) -> bool {
    err.contains("no se pudo conectar")
}

pub struct EnsureCatalogSynced<'a> {
    pub catalog: &'a dyn CatalogClient,
}

impl<'a> EnsureCatalogSynced<'a> {
    /// `session`: `Some((server_url, access))` si hay sesión activa; `None` = solo-local.
    pub fn run(
        &self,
        session: Option<(&str, &str)>,
        project: &CatalogProject,
        photos: &[CatalogPhoto],
    ) -> Result<SyncOutcome, String> {
        let Some((base, access)) = session else {
            return Ok(SyncOutcome::SkippedNoSession);
        };

        match self.catalog.project_exists(base, access, &project.id) {
            Ok(true) => Ok(SyncOutcome::AlreadyExists),
            Ok(false) => {
                self.catalog
                    .publish_projects(base, access, std::slice::from_ref(project))?;
                self.catalog.publish_photos(base, access, &project.id, photos)?;
                Ok(SyncOutcome::Published)
            }
            Err(e) if is_offline(&e) => Ok(SyncOutcome::Offline),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Fake configurable del `CatalogClient` (thread-safe: el trait exige Sync).
    struct FakeCatalog {
        exists: Result<bool, String>,
        published_projects: AtomicUsize,
        published_photos: AtomicUsize,
    }
    impl FakeCatalog {
        fn new(exists: Result<bool, String>) -> Self {
            Self {
                exists,
                published_projects: AtomicUsize::new(0),
                published_photos: AtomicUsize::new(0),
            }
        }
        fn projects_count(&self) -> usize {
            self.published_projects.load(Ordering::SeqCst)
        }
        fn photos_count(&self) -> usize {
            self.published_photos.load(Ordering::SeqCst)
        }
    }
    impl CatalogClient for FakeCatalog {
        fn project_exists(&self, _: &str, _: &str, _: &str) -> Result<bool, String> {
            self.exists.clone()
        }
        fn list_photos(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<Vec<crate::application::ports::CatalogPhotoStatus>, String> {
            Ok(vec![])
        }
        fn list_providers(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Vec<crate::application::ports::ProviderInfo>, String> {
            Ok(vec![])
        }
        fn upload_cover(&self, _: &str, _: &str, _: &str, _: &[u8]) -> Result<(), String> {
            Ok(())
        }
        fn publish_projects(&self, _: &str, _: &str, _: &[CatalogProject]) -> Result<(), String> {
            self.published_projects.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
        fn publish_photos(
            &self,
            _: &str,
            _: &str,
            _: &str,
            _: &[CatalogPhoto],
        ) -> Result<(), String> {
            self.published_photos.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn project() -> CatalogProject {
        CatalogProject {
            id: "p1".into(),
            name: "Boda".into(),
            session_date: Some("2024-06-15".into()),
            slug: "Boda".into(),
            workflow_status: None,
        }
    }

    #[test]
    fn no_session_is_skipped() {
        let fake = FakeCatalog::new(Ok(false));
        let uc = EnsureCatalogSynced { catalog: &fake };
        assert_eq!(uc.run(None, &project(), &[]).unwrap(), SyncOutcome::SkippedNoSession);
        assert_eq!(fake.projects_count(), 0);
    }

    #[test]
    fn publishes_when_missing() {
        let fake = FakeCatalog::new(Ok(false));
        let uc = EnsureCatalogSynced { catalog: &fake };
        let out = uc.run(Some(("http://s", "tok")), &project(), &[]).unwrap();
        assert_eq!(out, SyncOutcome::Published);
        assert_eq!(fake.projects_count(), 1);
        assert_eq!(fake.photos_count(), 1);
    }

    #[test]
    fn skips_publish_when_exists() {
        let fake = FakeCatalog::new(Ok(true));
        let uc = EnsureCatalogSynced { catalog: &fake };
        let out = uc.run(Some(("http://s", "tok")), &project(), &[]).unwrap();
        assert_eq!(out, SyncOutcome::AlreadyExists);
        assert_eq!(fake.projects_count(), 0);
    }

    #[test]
    fn offline_is_tolerated() {
        let fake = FakeCatalog::new(Err("no se pudo conectar con el server de respaldo".into()));
        let uc = EnsureCatalogSynced { catalog: &fake };
        let out = uc.run(Some(("http://s", "tok")), &project(), &[]).unwrap();
        assert_eq!(out, SyncOutcome::Offline);
        assert_eq!(fake.projects_count(), 0);
    }
}
