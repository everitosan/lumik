//! Implementación HTTP del puerto `BackupAuthClient` (Fase 4).
//! Habla con el server de respaldo (`apps/server`) vía `reqwest::blocking`.
//! Se usa desde comandos Tauri síncronos (corren en un thread pool, no en el
//! runtime async), por lo que el cliente bloqueante es seguro aquí.

use crate::application::ports::{AccountInfo, AuthTokens, BackupAuthClient};
use serde_json::{json, Value};

pub struct HttpBackupAuth;

impl HttpBackupAuth {
    fn client() -> Result<reqwest::blocking::Client, String> {
        reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| format!("no se pudo crear el cliente HTTP: {e}"))
    }
}

/// Envía un POST JSON y devuelve el JSON de respuesta, o un error legible
/// (usando el campo `error` del server cuando existe).
fn post_json(url: &str, body: Value, bearer: Option<&str>) -> Result<Value, String> {
    let client = HttpBackupAuth::client()?;
    let mut req = client.post(url).json(&body);
    if let Some(token) = bearer {
        req = req.bearer_auth(token);
    }
    let resp = req.send().map_err(|e| friendly_transport_error(&e))?;
    read_response(resp)
}

fn read_response(resp: reqwest::blocking::Response) -> Result<Value, String> {
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    let value: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    if status.is_success() {
        return Ok(value);
    }
    let msg = value
        .get("error")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("HTTP {}", status.as_u16()));
    Err(msg)
}

fn friendly_transport_error(e: &reqwest::Error) -> String {
    if e.is_connect() || e.is_timeout() {
        "no se pudo conectar con el server de respaldo".to_string()
    } else {
        format!("error de red: {e}")
    }
}

fn parse_tokens(v: &Value) -> Result<AuthTokens, String> {
    Ok(AuthTokens {
        access: str_field(v, "access")?,
        refresh: str_field(v, "refresh")?,
    })
}

fn parse_account(v: &Value) -> Result<AccountInfo, String> {
    Ok(AccountInfo {
        id: str_field(v, "id")?,
        email: str_field(v, "email")?,
        name: v.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()),
    })
}

fn str_field(v: &Value, key: &str) -> Result<String, String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("respuesta del server sin campo '{key}'"))
}

fn auth_url(base_url: &str, path: &str) -> String {
    format!("{}/v1/auth/{}", base_url.trim_end_matches('/'), path)
}

impl BackupAuthClient for HttpBackupAuth {
    fn register(
        &self,
        base_url: &str,
        email: &str,
        password: &str,
        name: Option<&str>,
    ) -> Result<(AuthTokens, AccountInfo), String> {
        let v = post_json(
            &auth_url(base_url, "register"),
            json!({ "email": email, "password": password, "name": name }),
            None,
        )?;
        Ok((parse_tokens(&v)?, parse_account(&v["photographer"])?))
    }

    fn login(
        &self,
        base_url: &str,
        email: &str,
        password: &str,
    ) -> Result<(AuthTokens, AccountInfo), String> {
        let v = post_json(
            &auth_url(base_url, "login"),
            json!({ "email": email, "password": password }),
            None,
        )?;
        Ok((parse_tokens(&v)?, parse_account(&v["photographer"])?))
    }

    fn me(&self, base_url: &str, access: &str) -> Result<AccountInfo, String> {
        let client = HttpBackupAuth::client()?;
        let resp = client
            .get(auth_url(base_url, "me"))
            .bearer_auth(access)
            .send()
            .map_err(|e| friendly_transport_error(&e))?;
        let v = read_response(resp)?;
        parse_account(&v)
    }

    fn refresh(&self, base_url: &str, refresh: &str) -> Result<AuthTokens, String> {
        let v = post_json(
            &auth_url(base_url, "refresh"),
            json!({ "refresh": refresh }),
            None,
        )?;
        parse_tokens(&v)
    }

    fn logout(&self, base_url: &str, refresh: &str) -> Result<(), String> {
        post_json(
            &auth_url(base_url, "logout"),
            json!({ "refresh": refresh }),
            None,
        )
        .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::BackupAuthClient;

    /// Flujo completo contra un server real. Se **salta** si `LUMIK_TEST_SERVER`
    /// no está definido (para no requerir red en CI).
    /// Correr: `LUMIK_TEST_SERVER=http://localhost:8787 cargo test -- --nocapture`
    #[test]
    fn auth_flow_against_server() {
        let Ok(base) = std::env::var("LUMIK_TEST_SERVER") else {
            eprintln!("(saltado: define LUMIK_TEST_SERVER para correr este test)");
            return;
        };
        let auth = HttpBackupAuth;
        let email = format!("desk-{}@lumik.mx", chrono::Utc::now().timestamp_micros());

        // register → tokens + cuenta
        let (tokens, acc) = auth
            .register(&base, &email, "password123", Some("Desk"))
            .expect("register");
        assert!(!tokens.access.is_empty());
        assert_eq!(acc.email, email);

        // me con el access
        let me = auth.me(&base, &tokens.access).expect("me");
        assert_eq!(me.id, acc.id);

        // login
        let (t2, _) = auth.login(&base, &email, "password123").expect("login");
        assert!(!t2.refresh.is_empty());

        // refresh rota el token
        let t3 = auth.refresh(&base, &t2.refresh).expect("refresh");
        assert!(!t3.access.is_empty());

        // credenciales incorrectas → error
        assert!(auth.login(&base, &email, "mala").is_err());
    }
}
