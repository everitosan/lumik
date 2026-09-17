//! Implementación HTTP del puerto `BackupClient` (Fase 7): sube el JPEG al server
//! por `multipart/form-data`; el server lo replica a la nube vía rclone.

use crate::application::ports::{BackupClient, BackupReceipt};
use serde_json::{json, Value};
use std::path::Path;

pub struct HttpBackup;

fn friendly(e: &reqwest::Error) -> String {
    if e.is_connect() || e.is_timeout() {
        "no se pudo conectar con el server de respaldo".to_string()
    } else {
        format!("error de red: {e}")
    }
}

impl BackupClient for HttpBackup {
    fn prepare_dir(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
        targets: Option<&str>,
    ) -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("cliente HTTP: {e}"))?;
        let url = format!(
            "{}/v1/projects/{}/backup/prepare",
            base_url.trim_end_matches('/'),
            project_id
        );
        let resp = client
            .post(url)
            .bearer_auth(access)
            .json(&json!({ "targets": targets }))
            .send()
            .map_err(|e| friendly(&e))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().unwrap_or_default();
            let msg = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            return Err(msg);
        }
        Ok(())
    }

    fn relocate_dir(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
    ) -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120)) // el move puede tardar
            .build()
            .map_err(|e| format!("cliente HTTP: {e}"))?;
        let url = format!(
            "{}/v1/projects/{}/backup/relocate",
            base_url.trim_end_matches('/'),
            project_id
        );
        let resp = client
            .post(url)
            .bearer_auth(access)
            .send()
            .map_err(|e| friendly(&e))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().unwrap_or_default();
            let msg = serde_json::from_str::<Value>(&text)
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(String::from))
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            return Err(msg);
        }
        Ok(())
    }

    fn folder_link(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
        provider: &str,
    ) -> Result<Option<String>, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| format!("cliente HTTP: {e}"))?;
        let url = format!(
            "{}/v1/projects/{}/backup/link",
            base_url.trim_end_matches('/'),
            project_id
        );
        let resp = client
            .post(url)
            .bearer_auth(access)
            .json(&json!({ "provider": provider }))
            .send()
            .map_err(|e| friendly(&e))?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status().as_u16()));
        }
        let v: Value = resp.json().map_err(|e| format!("respuesta inválida: {e}"))?;
        Ok(v.get("url").and_then(|u| u.as_str()).map(String::from))
    }

    fn upload_photo(
        &self,
        base_url: &str,
        access: &str,
        project_id: &str,
        photo_id: &str,
        file_path: &Path,
        filename: &str,
        targets: Option<&str>,
        quality: Option<&str>,
    ) -> Result<BackupReceipt, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // fotos grandes
            .build()
            .map_err(|e| format!("cliente HTTP: {e}"))?;

        let mut form = reqwest::blocking::multipart::Form::new()
            .file("file", file_path)
            .map_err(|e| format!("no se pudo leer el archivo: {e}"))?
            .text("filename", filename.to_string());
        if let Some(t) = targets {
            form = form.text("targets", t.to_string());
        }
        if let Some(q) = quality {
            form = form.text("quality", q.to_string());
        }

        let url = format!(
            "{}/v1/projects/{}/photos/{}/backup",
            base_url.trim_end_matches('/'),
            project_id,
            photo_id
        );
        let resp = client
            .post(url)
            .bearer_auth(access)
            .multipart(form)
            .send()
            .map_err(|e| friendly(&e))?;

        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);

        if !status.is_success() {
            let msg = v
                .get("error")
                .and_then(|e| e.as_str())
                .map(String::from)
                .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
            return Err(msg);
        }

        let backup_status = v
            .get("backup_status")
            .and_then(|s| s.as_str())
            .unwrap_or("failed")
            .to_string();
        // Primer resultado exitoso con su ruta remota.
        let backup_url = v
            .get("results")
            .and_then(|r| r.as_array())
            .and_then(|arr| {
                arr.iter()
                    .find(|r| r.get("status").and_then(|s| s.as_str()) == Some("uploaded"))
                    .and_then(|r| r.get("remote_path").and_then(|p| p.as_str()))
                    .map(String::from)
            });

        Ok(BackupReceipt {
            status: backup_status,
            backup_url,
        })
    }
}
