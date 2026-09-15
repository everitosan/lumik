//! Test de integración del endpoint de backup (Fase 7).
//! Usa un remote `alias` local (sin credenciales de nube reales).

use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use lumik_server::{build_router, config::Config, crypto::Crypto, AppState};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

const KEY: &str = "0000000000000000000000000000000000000000000000000000000000000001";

fn test_config() -> Config {
    Config {
        database_url: String::new(),
        bind_addr: String::new(),
        // Usa el binario local si existe; si no, el del PATH.
        rclone_bin: if std::path::Path::new("./bin/rclone").exists() {
            "./bin/rclone".into()
        } else {
            "rclone".into()
        },
        jwt_secret: "test-secret".into(),
        access_ttl_secs: 900,
        refresh_ttl_secs: 3600,
        rate_max: 1000,
        rate_window_secs: 60,
        remote_enc_key: KEY.into(),
    }
}

fn app(pool: PgPool) -> Router {
    build_router(AppState::new(pool, test_config()))
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

/// Registra y devuelve (access, photographer_id).
async fn register(app: &Router, email: &str) -> (String, String) {
    let resp = app
        .clone()
        .oneshot(
            Request::post("/v1/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "email": email, "password": "password123" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let v = json_body(resp).await;
    (
        v["access"].as_str().unwrap().to_string(),
        v["photographer"]["id"].as_str().unwrap().to_string(),
    )
}

fn post_json(uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::post(uri)
        .header("content-type", "application/json")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[sqlx::test]
async fn backup_uploads_via_alias_and_updates_catalog(pool: PgPool) {
    // rclone debe existir para este test.
    if Crypto::new(KEY).is_err() {
        return;
    }
    let cfg = test_config();
    if lumik_server::rclone::Rclone::new(&cfg.rclone_bin)
        .version()
        .is_err()
    {
        eprintln!("(saltado: rclone no disponible)");
        return;
    }

    let app = app(pool.clone());
    let (token, photographer_id) = register(&app, "bk@lumik.mx").await;
    let pid = Uuid::new_v4();
    let photo_id = Uuid::new_v4();

    // Catálogo: proyecto + foto.
    app.clone()
        .oneshot(post_json(
            "/v1/sync/projects",
            &token,
            json!([{ "id": pid, "name": "Boda", "session_date": "2024-06-15", "slug": "Boda" }]),
        ))
        .await
        .unwrap();
    // Nota: NO se pre-sincroniza la foto; el endpoint de backup la upsertea
    // (permite respaldar archivos de _exported sin fila previa).

    // Remote 'alias' local, insertado directo (cifrado con la clave de test).
    let dest_root = std::env::temp_dir().join(format!("lumik-bk-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dest_root).unwrap();
    let crypto = Crypto::new(KEY).unwrap();
    let token_enc = crypto
        .encrypt(&json!({ "type": "alias", "remote": dest_root.to_string_lossy() }).to_string())
        .unwrap();
    sqlx::query(
        "INSERT INTO remote (photographer_id, provider, name, token) VALUES ($1, 'mega', 'test_alias', $2)",
    )
    .bind(Uuid::parse_str(&photographer_id).unwrap())
    .bind(&token_enc)
    .execute(&pool)
    .await
    .unwrap();

    // POST backup (multipart con un archivo).
    let boundary = "XBOUNDARY";
    let file_content = "contenido de la foto";
    let body = format!(
        "--{b}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"IMG_1.jpg\"\r\n\
         Content-Type: application/octet-stream\r\n\r\n{c}\r\n--{b}--\r\n",
        b = boundary,
        c = file_content
    );
    let resp = app
        .clone()
        .oneshot(
            Request::post(format!("/v1/projects/{pid}/photos/{photo_id}/backup"))
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v = json_body(resp).await;
    assert_eq!(v["backup_status"], "uploaded");

    // El archivo aterrizó en la ruta espejo.
    let landed = dest_root.join("Lumik/2024/06/15_Boda/_exported/IMG_1.jpg");
    assert!(landed.exists(), "el archivo debió subirse vía rclone: {landed:?}");
    assert_eq!(std::fs::read_to_string(&landed).unwrap(), file_content);

    // El catálogo refleja 'uploaded'.
    let resp = app
        .oneshot(
            Request::get(format!("/v1/projects/{pid}/photos"))
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let photos = json_body(resp).await;
    assert_eq!(photos[0]["backup_status"], "uploaded");
    assert!(photos[0]["backup_url"].as_str().unwrap().contains("_exported/IMG_1.jpg"));

    let _ = std::fs::remove_dir_all(&dest_root);
}
