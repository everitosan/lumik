//! Implementación HTTP del puerto `CatalogClient` (Fase 5).
//! Publica proyectos/fotos al catálogo del server y consulta existencia.

use crate::application::ports::{
    CatalogClient, CatalogPhoto, CatalogPhotoStatus, CatalogProject, ProviderInfo,
};
use serde_json::{json, Value};

pub struct HttpCatalog;

impl HttpCatalog {
    fn client() -> Result<reqwest::blocking::Client, String> {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("no se pudo crear el cliente HTTP: {e}"))
    }
}

fn friendly(e: &reqwest::Error) -> String {
    if e.is_connect() || e.is_timeout() {
        "no se pudo conectar con el server de respaldo".to_string()
    } else {
        format!("error de red: {e}")
    }
}

/// Lee la respuesta: `Ok(())` si 2xx, error con el `error` del server si no.
fn expect_success(resp: reqwest::blocking::Response) -> Result<(), String> {
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let text = resp.text().unwrap_or_default();
    let msg = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    Err(msg)
}

fn base(base_url: &str) -> String {
    base_url.trim_end_matches('/').to_string()
}

impl CatalogClient for HttpCatalog {
    fn project_exists(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
    ) -> Result<bool, String> {
        let url = format!("{}/v1/projects/{}", base(base_url), project_id);
        let resp = HttpCatalog::client()?
            .get(url)
            .bearer_auth(access)
            .send()
            .map_err(|e| friendly(&e))?;
        match resp.status().as_u16() {
            200 => Ok(true),
            404 => Ok(false),
            _ => Err(expect_success(resp).unwrap_err()),
        }
    }

    fn list_photos(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
    ) -> Result<Vec<CatalogPhotoStatus>, String> {
        let url = format!("{}/v1/projects/{}/photos", base(base_url), project_id);
        let resp = HttpCatalog::client()?
            .get(url)
            .bearer_auth(access)
            .send()
            .map_err(|e| friendly(&e))?;
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        if !status.is_success() {
            let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
            let msg = v
                .get("error")
                .and_then(|e| e.as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            return Err(msg);
        }
        let arr: Vec<Value> = serde_json::from_str(&text).unwrap_or_default();
        Ok(arr
            .into_iter()
            .filter_map(|v| {
                Some(CatalogPhotoStatus {
                    id: v.get("id")?.as_str()?.to_string(),
                    backup_status: v
                        .get("backup_status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("pending")
                        .to_string(),
                })
            })
            .collect())
    }

    fn list_providers(&self, base_url: &str, access: &str) -> Result<Vec<ProviderInfo>, String> {
        let url = format!("{}/v1/providers", base(base_url));
        let resp = HttpCatalog::client()?
            .get(url)
            .bearer_auth(access)
            .send()
            .map_err(|e| friendly(&e))?;
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        if !status.is_success() {
            return Err(format!("HTTP {}", status.as_u16()));
        }
        let arr: Vec<Value> = serde_json::from_str(&text).unwrap_or_default();
        Ok(arr
            .into_iter()
            .filter_map(|v| {
                Some(ProviderInfo {
                    provider: v.get("provider")?.as_str()?.to_string(),
                    name: v.get("name")?.as_str()?.to_string(),
                })
            })
            .collect())
    }

    fn publish_projects(
        &self,
        base_url: &str,
        access: &str,
        projects: &[CatalogProject],
    ) -> Result<(), String> {
        let body: Vec<Value> = projects
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "name": p.name,
                    "session_date": p.session_date,
                    "slug": p.slug,
                })
            })
            .collect();
        let resp = HttpCatalog::client()?
            .post(format!("{}/v1/sync/projects", base(base_url)))
            .bearer_auth(access)
            .json(&body)
            .send()
            .map_err(|e| friendly(&e))?;
        expect_success(resp)
    }

    fn publish_photos(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
        photos: &[CatalogPhoto],
    ) -> Result<(), String> {
        let body: Vec<Value> = photos
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "filename": p.filename,
                    "file_hash": p.file_hash,
                    "jpg_path": p.jpg_path,
                })
            })
            .collect();
        let url = format!("{}/v1/sync/projects/{}/photos", base(base_url), project_id);
        let resp = HttpCatalog::client()?
            .post(url)
            .bearer_auth(access)
            .json(&body)
            .send()
            .map_err(|e| friendly(&e))?;
        expect_success(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::BackupAuthClient;
    use crate::infrastructure::backup_auth::HttpBackupAuth;

    /// Publica un proyecto al catálogo real y comprueba `project_exists`.
    /// Se **salta** si `LUMIK_TEST_SERVER` no está definido.
    #[test]
    fn publish_and_exists_against_server() {
        let Ok(base) = std::env::var("LUMIK_TEST_SERVER") else {
            eprintln!("(saltado: define LUMIK_TEST_SERVER)");
            return;
        };
        let email = format!("catdesk-{}@lumik.mx", chrono::Utc::now().timestamp_micros());
        let (tokens, _) = HttpBackupAuth
            .register(&base, &email, "password123", None)
            .expect("register");

        let cat = HttpCatalog;
        let pid = uuid::Uuid::new_v4().to_string();

        // Antes de publicar no existe.
        assert!(!cat.project_exists(&base, &tokens.access, &pid).unwrap());

        // Publica proyecto + una foto.
        cat.publish_projects(
            &base,
            &tokens.access,
            &[CatalogProject {
                id: pid.clone(),
                name: "Boda".into(),
                session_date: Some("2024-06-15".into()),
                slug: "Boda".into(),
            }],
        )
        .expect("publish_projects");
        cat.publish_photos(
            &base,
            &tokens.access,
            &pid,
            &[CatalogPhoto {
                id: uuid::Uuid::new_v4().to_string(),
                filename: "IMG_1.jpg".into(),
                file_hash: None,
                jpg_path: Some("_exported/IMG_1.jpg".into()),
            }],
        )
        .expect("publish_photos");

        // Ahora sí existe.
        assert!(cat.project_exists(&base, &tokens.access, &pid).unwrap());
    }
}
