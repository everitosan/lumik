//! Tests de integración de la conexión de proveedores (Fase 3).

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

/// Registra un fotógrafo y devuelve su access token.
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

fn post_json(uri: &str, token: &str, body: Value) -> Request<Body> {
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
async fn connect_nextcloud_then_list(pool: PgPool) {
    let app = app(pool);
    let token = register(&app, "np@lumik.mx").await;

    let resp = app
        .clone()
        .oneshot(post_json(
            "/v1/providers/nextcloud/connect",
            &token,
            json!({ "url": "https://nc.example.com/remote.php/dav", "username": "u", "password": "p" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let dto = json_body(resp).await;
    assert_eq!(dto["provider"], "nextcloud");
    assert!(dto["name"].as_str().unwrap().ends_with("_nextcloud"));

    // La lista lo incluye y NUNCA expone credenciales.
    let resp = app.oneshot(get("/v1/providers", &token)).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let list = json_body(resp).await;
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert!(list[0].get("token").is_none());
    assert!(list[0].get("password").is_none());
}

#[sqlx::test]
async fn connect_requires_fields(pool: PgPool) {
    let app = app(pool);
    let token = register(&app, "mf@lumik.mx").await;
    // mega sin password → 400
    let resp = app
        .oneshot(post_json(
            "/v1/providers/mega/connect",
            &token,
            json!({ "email": "x@y.z" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test]
async fn oauth_provider_is_gated(pool: PgPool) {
    let app = app(pool);
    let token = register(&app, "oa@lumik.mx").await;
    let resp = app
        .oneshot(post_json("/v1/providers/gdrive/connect", &token, json!({})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
}

#[sqlx::test]
async fn connect_requires_auth(pool: PgPool) {
    let app = app(pool);
    let resp = app
        .oneshot(
            Request::post("/v1/providers/mega/connect")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "email": "a", "password": "b" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn delete_provider(pool: PgPool) {
    let app = app(pool);
    let token = register(&app, "del@lumik.mx").await;
    let resp = app
        .clone()
        .oneshot(post_json(
            "/v1/providers/mega/connect",
            &token,
            json!({ "email": "a@b.c", "password": "secret" }),
        ))
        .await
        .unwrap();
    let id = json_body(resp).await["id"].as_str().unwrap().to_string();

    let resp = app
        .clone()
        .oneshot(
            Request::delete(format!("/v1/providers/{id}"))
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let resp = app.oneshot(get("/v1/providers", &token)).await.unwrap();
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn tenant_isolation_providers(pool: PgPool) {
    let app = app(pool);
    let a = register(&app, "iso-a@lumik.mx").await;
    let b = register(&app, "iso-b@lumik.mx").await;

    // A conecta un proveedor.
    app.clone()
        .oneshot(post_json(
            "/v1/providers/mega/connect",
            &a,
            json!({ "email": "a@b.c", "password": "secret" }),
        ))
        .await
        .unwrap();

    // B no ve nada de A.
    let resp = app.clone().oneshot(get("/v1/providers", &b)).await.unwrap();
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 0);

    // A sí ve el suyo.
    let resp = app.oneshot(get("/v1/providers", &a)).await.unwrap();
    assert_eq!(json_body(resp).await.as_array().unwrap().len(), 1);
}
