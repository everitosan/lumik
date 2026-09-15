//! Tests de integración del catálogo (Fase 5).

use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use lumik_server::{build_router, config::Config, AppState};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

fn test_config() -> Config {
    Config {
        database_url: String::new(),
        bind_addr: String::new(),
        rclone_bin: "rclone".into(),
        jwt_secret: "test-secret".into(),
        access_ttl_secs: 900,
        refresh_ttl_secs: 3600,
        rate_max: 1000,
        rate_window_secs: 60,
        remote_enc_key: "0000000000000000000000000000000000000000000000000000000000000001"
            .into(),
    }
}

fn app(pool: PgPool) -> Router {
    build_router(AppState::new(pool, test_config()))
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn register(app: &Router, email: &str) -> String {
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
    assert_eq!(resp.status(), StatusCode::OK);
    json_body(resp).await["access"].as_str().unwrap().to_string()
}

fn post(uri: &str, token: &str, body: Value) -> Request<Body> {
    Request::post(uri)
        .header("content-type", "application/json")
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(uri: &str, token: &str) -> Request<Body> {
    Request::get(uri)
        .header(AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap()
}

#[sqlx::test]
async fn sync_project_then_exists_and_list(pool: PgPool) {
    let app = app(pool);
    let token = register(&app, "cat-a@lumik.mx").await;
    let pid = Uuid::new_v4();

    // sync
    let resp = app
        .clone()
        .oneshot(post(
            "/v1/sync/projects",
            &token,
            json!([{ "id": pid, "name": "Boda", "session_date": "2024-06-15", "slug": "15_Boda" }]),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["synced"], 1);

    // exists → 200
    let resp = app
        .clone()
        .oneshot(get(&format!("/v1/projects/{pid}"), &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["name"], "Boda");

    // random → 404
    let resp = app
        .clone()
        .oneshot(get(&format!("/v1/projects/{}", Uuid::new_v4()), &token))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // list incluye el proyecto
    let resp = app.oneshot(get("/v1/projects", &token)).await.unwrap();
    let list = json_body(resp).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["slug"], "15_Boda");
}

#[sqlx::test]
async fn sync_photos_preserves_backup_status(pool: PgPool) {
    let app = app(pool.clone());
    let token = register(&app, "cat-p@lumik.mx").await;
    let pid = Uuid::new_v4();
    let photo_id = Uuid::new_v4();

    app.clone()
        .oneshot(post(
            "/v1/sync/projects",
            &token,
            json!([{ "id": pid, "name": "P", "session_date": null, "slug": "P" }]),
        ))
        .await
        .unwrap();

    // Primer sync de la foto (pending por defecto).
    app.clone()
        .oneshot(post(
            &format!("/v1/sync/projects/{pid}/photos"),
            &token,
            json!([{ "id": photo_id, "filename": "IMG_1.jpg", "file_hash": null, "jpg_path": null }]),
        ))
        .await
        .unwrap();

    // El server marca la foto como respaldada (simula un backup).
    sqlx::query("UPDATE photo SET backup_status = 'uploaded', backup_url = 'x://u' WHERE id = $1")
        .bind(photo_id)
        .execute(&pool)
        .await
        .unwrap();

    // Re-sync de metadata desde la desktop (jpg_path ahora presente).
    app.clone()
        .oneshot(post(
            &format!("/v1/sync/projects/{pid}/photos"),
            &token,
            json!([{ "id": photo_id, "filename": "IMG_1.jpg", "file_hash": "abc", "jpg_path": "_exported/IMG_1.jpg" }]),
        ))
        .await
        .unwrap();

    // El backup_status NO se pisó; la metadata sí se actualizó.
    let resp = app
        .oneshot(get(&format!("/v1/projects/{pid}/photos"), &token))
        .await
        .unwrap();
    let photos = json_body(resp).await;
    assert_eq!(photos[0]["backup_status"], "uploaded");
    assert_eq!(photos[0]["backup_url"], "x://u");
    assert_eq!(photos[0]["jpg_path"], "_exported/IMG_1.jpg");
    assert_eq!(photos[0]["file_hash"], "abc");
}

#[sqlx::test]
async fn photos_pending_filter(pool: PgPool) {
    let app = app(pool);
    let token = register(&app, "cat-f@lumik.mx").await;
    let pid = Uuid::new_v4();
    app.clone()
        .oneshot(post(
            "/v1/sync/projects",
            &token,
            json!([{ "id": pid, "name": "P", "session_date": null, "slug": "P" }]),
        ))
        .await
        .unwrap();
    app.clone()
        .oneshot(post(
            &format!("/v1/sync/projects/{pid}/photos"),
            &token,
            json!([{ "id": Uuid::new_v4(), "filename": "a.jpg", "file_hash": null, "jpg_path": null }]),
        ))
        .await
        .unwrap();

    let resp = app
        .oneshot(get(&format!("/v1/projects/{pid}/photos?backup=pending"), &token))
        .await
        .unwrap();
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 1);
}

#[sqlx::test]
async fn tenant_isolation_catalog(pool: PgPool) {
    let app = app(pool);
    let a = register(&app, "cat-iso-a@lumik.mx").await;
    let b = register(&app, "cat-iso-b@lumik.mx").await;
    let pid = Uuid::new_v4();

    // A crea un proyecto.
    app.clone()
        .oneshot(post(
            "/v1/sync/projects",
            &a,
            json!([{ "id": pid, "name": "A", "session_date": null, "slug": "A" }]),
        ))
        .await
        .unwrap();

    // B no lo ve.
    let resp = app
        .clone()
        .oneshot(get(&format!("/v1/projects/{pid}"), &b))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = app.clone().oneshot(get("/v1/projects", &b)).await.unwrap();
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 0);

    // B no puede sincronizar fotos al proyecto de A → 404.
    let resp = app
        .oneshot(post(
            &format!("/v1/sync/projects/{pid}/photos"),
            &b,
            json!([{ "id": Uuid::new_v4(), "filename": "x.jpg", "file_hash": null, "jpg_path": null }]),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
