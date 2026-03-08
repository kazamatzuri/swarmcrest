// Integration tests for the bot and bot-version CRUD REST API endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt; // for oneshot()

/// Create a test app backed by an in-memory SQLite database.
/// Each call yields a completely fresh database so tests are isolated.
async fn test_app() -> axum::Router {
    sqlx::any::install_default_drivers();
    let db = swarmcrest_backend::db::Database::new("sqlite::memory:")
        .await
        .unwrap();
    let db = Arc::new(db);
    let game_server = Arc::new(swarmcrest_backend::engine::server::GameServer::new());
    let rate_limiter = swarmcrest_backend::rate_limit::RateLimiter::new();

    // Enable local mode so the AuthUser extractor auto-authenticates.
    swarmcrest_backend::config::set_local_mode(true);

    // Ensure the local user exists in this fresh database.
    let password_hash = swarmcrest_backend::auth::hash_password("local-mode-password").unwrap();
    db.create_user(
        swarmcrest_backend::config::LOCAL_USERNAME,
        "local@localhost",
        &password_hash,
        "Local Player",
    )
    .await
    .unwrap();

    let maps_dir = std::path::PathBuf::from("../data/maps");
    swarmcrest_backend::api::router(db, game_server, rate_limiter, maps_dir)
}

/// Parse the response body as JSON.
async fn body_json(response: axum::http::Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ---------------------------------------------------------------------------
// Helper: build common requests
// ---------------------------------------------------------------------------

fn post_json(uri: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap()
}

fn put_json(uri: &str, body: &Value) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("PUT")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(body).unwrap()))
        .unwrap()
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("GET")
        .body(Body::empty())
        .unwrap()
}

fn delete_request(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("DELETE")
        .body(Body::empty())
        .unwrap()
}

// ---------------------------------------------------------------------------
// Bot CRUD tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_bot() {
    let app = test_app().await;
    let req = post_json("/api/bots", &json!({"name": "TestBot"}));
    let resp = app.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "TestBot");
    assert!(body["id"].as_i64().is_some());
}

#[tokio::test]
async fn test_create_bot_empty_name() {
    let app = test_app().await;
    let req = post_json("/api/bots", &json!({"name": ""}));
    let resp = app.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_bots() {
    let app = test_app().await;

    // Create two bots.
    let resp = app
        .clone()
        .oneshot(post_json("/api/bots", &json!({"name": "Bot1"})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(post_json("/api/bots", &json!({"name": "Bot2"})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List bots (owned by the local user).
    let resp = app.clone().oneshot(get_request("/api/bots")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let bots = body.as_array().expect("expected an array");
    assert_eq!(bots.len(), 2);
}

#[tokio::test]
async fn test_get_bot() {
    let app = test_app().await;

    let resp = app
        .clone()
        .oneshot(post_json("/api/bots", &json!({"name": "GetMe"})))
        .await
        .unwrap();
    let created = body_json(resp).await;
    let id = created["id"].as_i64().unwrap();

    let resp = app
        .clone()
        .oneshot(get_request(&format!("/api/bots/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "GetMe");
}

#[tokio::test]
async fn test_get_bot_not_found() {
    let app = test_app().await;
    let resp = app.oneshot(get_request("/api/bots/99999")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_bot() {
    let app = test_app().await;

    let resp = app
        .clone()
        .oneshot(post_json("/api/bots", &json!({"name": "OldName"})))
        .await
        .unwrap();
    let created = body_json(resp).await;
    let id = created["id"].as_i64().unwrap();

    let resp = app
        .clone()
        .oneshot(put_json(
            &format!("/api/bots/{id}"),
            &json!({"name": "NewName"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "NewName");
}

#[tokio::test]
async fn test_update_bot_not_found() {
    let app = test_app().await;
    let resp = app
        .oneshot(put_json("/api/bots/99999", &json!({"name": "Whatever"})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_bot() {
    let app = test_app().await;

    let resp = app
        .clone()
        .oneshot(post_json("/api/bots", &json!({"name": "DeleteMe"})))
        .await
        .unwrap();
    let created = body_json(resp).await;
    let id = created["id"].as_i64().unwrap();

    let resp = app
        .clone()
        .oneshot(delete_request(&format!("/api/bots/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Confirm it is gone.
    let resp = app
        .clone()
        .oneshot(get_request(&format!("/api/bots/{id}")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_delete_bot_not_found() {
    let app = test_app().await;
    let resp = app
        .oneshot(delete_request("/api/bots/99999"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ---------------------------------------------------------------------------
// Bot version tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_create_bot_version() {
    let app = test_app().await;

    // Create a bot first.
    let resp = app
        .clone()
        .oneshot(post_json("/api/bots", &json!({"name": "VersionBot"})))
        .await
        .unwrap();
    let bot = body_json(resp).await;
    let bot_id = bot["id"].as_i64().unwrap();

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/api/bots/{bot_id}/versions"),
            &json!({"code": "function Creature:main() end"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert!(body["id"].as_i64().is_some());
}

#[tokio::test]
async fn test_create_bot_version_too_large() {
    let app = test_app().await;

    let resp = app
        .clone()
        .oneshot(post_json("/api/bots", &json!({"name": "BigBot"})))
        .await
        .unwrap();
    let bot = body_json(resp).await;
    let bot_id = bot["id"].as_i64().unwrap();

    // Build a code string that exceeds 64 KB.
    let big_code = "x".repeat(64 * 1024 + 1);
    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/api/bots/{bot_id}/versions"),
            &json!({"code": big_code}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_list_bot_versions() {
    let app = test_app().await;

    let resp = app
        .clone()
        .oneshot(post_json("/api/bots", &json!({"name": "MultiVer"})))
        .await
        .unwrap();
    let bot = body_json(resp).await;
    let bot_id = bot["id"].as_i64().unwrap();

    // Create two versions.
    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/api/bots/{bot_id}/versions"),
            &json!({"code": "-- v1"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/api/bots/{bot_id}/versions"),
            &json!({"code": "-- v2"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(get_request(&format!("/api/bots/{bot_id}/versions")))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let versions = body.as_array().expect("expected an array");
    assert_eq!(versions.len(), 2);
}

#[tokio::test]
async fn test_set_active_version() {
    let app = test_app().await;

    let resp = app
        .clone()
        .oneshot(post_json("/api/bots", &json!({"name": "ActiveBot"})))
        .await
        .unwrap();
    let bot = body_json(resp).await;
    let bot_id = bot["id"].as_i64().unwrap();

    // Create a version.
    let resp = app
        .clone()
        .oneshot(post_json(
            &format!("/api/bots/{bot_id}/versions"),
            &json!({"code": "-- active"}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let version = body_json(resp).await;
    let version_id = version["id"].as_i64().unwrap();

    // Set it as the active version.
    let resp = app
        .clone()
        .oneshot(put_json(
            &format!("/api/bots/{bot_id}/active-version"),
            &json!({"version_id": version_id}),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["active_version_id"], version_id);
}
