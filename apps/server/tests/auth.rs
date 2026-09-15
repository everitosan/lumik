//! Tests de integración de la capa de auth (Fase 1).
//! `#[sqlx::test]` crea una BD efímera por test y aplica las migraciones.

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
        jwt_secret: "test-secret-please-change".into(),
        access_ttl_secs: 900,
        refresh_ttl_secs: 3600,
        rate_max: 1000, // alto para no interferir en los tests
        rate_window_secs: 60,
    }
}

fn app(pool: PgPool) -> Router {
    build_router(AppState::new(pool, test_config()))
}

fn post_json(uri: &str, body: Value) -> Request<Body> {
    Request::post(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn json_body(resp: axum::response::Response) -> Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

async fn register(app: &Router, email: &str, pw: &str) -> Value {
    let resp = app
        .clone()
        .oneshot(post_json(
            "/v1/auth/register",
            json!({ "email": email, "password": pw }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "register debería ser 200");
    json_body(resp).await
}

#[sqlx::test]
async fn register_then_me(pool: PgPool) {
    let app = app(pool);
    let reg = register(&app, "a@lumik.mx", "password123").await;
    let access = reg["access"].as_str().unwrap().to_string();
    assert_eq!(reg["photographer"]["email"], "a@lumik.mx");

    // /me con token válido
    let resp = app
        .clone()
        .oneshot(
            Request::get("/v1/auth/me")
                .header(AUTHORIZATION, format!("Bearer {access}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["email"], "a@lumik.mx");
}

#[sqlx::test]
async fn me_without_token_is_401(pool: PgPool) {
    let app = app(pool);
    let resp = app
        .oneshot(Request::get("/v1/auth/me").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn duplicate_email_is_409(pool: PgPool) {
    let app = app(pool);
    register(&app, "dup@lumik.mx", "password123").await;
    let resp = app
        .oneshot(post_json(
            "/v1/auth/register",
            json!({ "email": "dup@lumik.mx", "password": "password123" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[sqlx::test]
async fn login_ok_and_wrong_password(pool: PgPool) {
    let app = app(pool);
    register(&app, "login@lumik.mx", "password123").await;

    // login correcto
    let resp = app
        .clone()
        .oneshot(post_json(
            "/v1/auth/login",
            json!({ "email": "login@lumik.mx", "password": "password123" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // contraseña incorrecta
    let resp = app
        .oneshot(post_json(
            "/v1/auth/login",
            json!({ "email": "login@lumik.mx", "password": "incorrecta" }),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn refresh_rotates_and_old_token_is_rejected(pool: PgPool) {
    let app = app(pool);
    let reg = register(&app, "refresh@lumik.mx", "password123").await;
    let refresh1 = reg["refresh"].as_str().unwrap().to_string();

    // refresh con el token original → nuevo par
    let resp = app
        .clone()
        .oneshot(post_json("/v1/auth/refresh", json!({ "refresh": refresh1 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let pair = json_body(resp).await;
    assert!(pair["access"].as_str().is_some());

    // reusar el refresh viejo → 401 (fue rotado)
    let resp = app
        .oneshot(post_json("/v1/auth/refresh", json!({ "refresh": refresh1 })))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn tenant_isolation_me_returns_own_identity(pool: PgPool) {
    let app = app(pool);
    let a = register(&app, "tenant-a@lumik.mx", "password123").await;
    let b = register(&app, "tenant-b@lumik.mx", "password123").await;
    let access_a = a["access"].as_str().unwrap().to_string();
    let access_b = b["access"].as_str().unwrap().to_string();

    let me = |token: String| {
        let app = app.clone();
        async move {
            let resp = app
                .oneshot(
                    Request::get("/v1/auth/me")
                        .header(AUTHORIZATION, format!("Bearer {token}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            json_body(resp).await
        }
    };

    let me_a = me(access_a).await;
    let me_b = me(access_b).await;

    // Cada token resuelve SOLO a su propio fotógrafo (base del aislamiento).
    assert_eq!(me_a["email"], "tenant-a@lumik.mx");
    assert_eq!(me_b["email"], "tenant-b@lumik.mx");
    assert_ne!(me_a["id"], me_b["id"]);
}
