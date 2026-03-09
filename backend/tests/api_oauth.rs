// Integration tests for OAuth SSO endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use axum::Router;
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

/// Create a test app with OAuth routes wired up.
/// Configures OAuth state with no providers (to test disabled state)
/// and optionally with mock providers.
async fn test_app_no_oauth() -> Router {
    sqlx::any::install_default_drivers();
    let db = swarmcrest_backend::db::Database::new("sqlite::memory:")
        .await
        .unwrap();
    let db = Arc::new(db);

    swarmcrest_backend::config::set_local_mode(true);

    let password_hash = swarmcrest_backend::auth::hash_password("local-mode-password").unwrap();
    db.create_user(
        swarmcrest_backend::config::LOCAL_USERNAME,
        "local@localhost",
        &password_hash,
        "Local Player",
    )
    .await
    .unwrap();

    // Config with no OAuth providers configured
    let cfg = swarmcrest_backend::config::Config {
        database_url: "sqlite::memory:".to_string(),
        port: 3000,
        maps_dir: std::path::PathBuf::from("../data/maps"),
        local_mode: true,
        password_auth_enabled: true,
        static_dir: None,
        worker_count: 1,
        queue_poll_ms: 1000,
        github_client_id: None,
        github_client_secret: None,
        google_client_id: None,
        google_client_secret: None,
        oauth_redirect_base: "http://localhost:3000".to_string(),
    };

    let oauth_state = Arc::new(swarmcrest_backend::oauth::OAuthState::new(&cfg, db));

    Router::new()
        .route(
            "/api/auth/providers",
            get(swarmcrest_backend::oauth::auth_providers),
        )
        .route(
            "/api/auth/oauth/github",
            get(swarmcrest_backend::oauth::github_auth_start),
        )
        .route(
            "/api/auth/oauth/google",
            get(swarmcrest_backend::oauth::google_auth_start),
        )
        .with_state(oauth_state)
}

async fn test_app_with_github() -> Router {
    sqlx::any::install_default_drivers();
    let db = swarmcrest_backend::db::Database::new("sqlite::memory:")
        .await
        .unwrap();
    let db = Arc::new(db);

    swarmcrest_backend::config::set_local_mode(true);

    let password_hash = swarmcrest_backend::auth::hash_password("local-mode-password").unwrap();
    db.create_user(
        swarmcrest_backend::config::LOCAL_USERNAME,
        "local@localhost",
        &password_hash,
        "Local Player",
    )
    .await
    .unwrap();

    let cfg = swarmcrest_backend::config::Config {
        database_url: "sqlite::memory:".to_string(),
        port: 3000,
        maps_dir: std::path::PathBuf::from("../data/maps"),
        local_mode: true,
        password_auth_enabled: true,
        static_dir: None,
        worker_count: 1,
        queue_poll_ms: 1000,
        github_client_id: Some("test-github-id".to_string()),
        github_client_secret: Some("test-github-secret".to_string()),
        google_client_id: None,
        google_client_secret: None,
        oauth_redirect_base: "http://localhost:3000".to_string(),
    };

    let oauth_state = Arc::new(swarmcrest_backend::oauth::OAuthState::new(&cfg, db));

    Router::new()
        .route(
            "/api/auth/providers",
            get(swarmcrest_backend::oauth::auth_providers),
        )
        .route(
            "/api/auth/oauth/github",
            get(swarmcrest_backend::oauth::github_auth_start),
        )
        .route(
            "/api/auth/oauth/google",
            get(swarmcrest_backend::oauth::google_auth_start),
        )
        .with_state(oauth_state)
}

async fn test_app_with_both() -> Router {
    sqlx::any::install_default_drivers();
    let db = swarmcrest_backend::db::Database::new("sqlite::memory:")
        .await
        .unwrap();
    let db = Arc::new(db);

    swarmcrest_backend::config::set_local_mode(true);

    let password_hash = swarmcrest_backend::auth::hash_password("local-mode-password").unwrap();
    db.create_user(
        swarmcrest_backend::config::LOCAL_USERNAME,
        "local@localhost",
        &password_hash,
        "Local Player",
    )
    .await
    .unwrap();

    let cfg = swarmcrest_backend::config::Config {
        database_url: "sqlite::memory:".to_string(),
        port: 3000,
        maps_dir: std::path::PathBuf::from("../data/maps"),
        local_mode: true,
        password_auth_enabled: true,
        static_dir: None,
        worker_count: 1,
        queue_poll_ms: 1000,
        github_client_id: Some("test-github-id".to_string()),
        github_client_secret: Some("test-github-secret".to_string()),
        google_client_id: Some("test-google-id".to_string()),
        google_client_secret: Some("test-google-secret".to_string()),
        oauth_redirect_base: "http://localhost:3000".to_string(),
    };

    let oauth_state = Arc::new(swarmcrest_backend::oauth::OAuthState::new(&cfg, db));

    Router::new()
        .route(
            "/api/auth/providers",
            get(swarmcrest_backend::oauth::auth_providers),
        )
        .route(
            "/api/auth/oauth/github",
            get(swarmcrest_backend::oauth::github_auth_start),
        )
        .route(
            "/api/auth/oauth/google",
            get(swarmcrest_backend::oauth::google_auth_start),
        )
        .with_state(oauth_state)
}

fn get_request(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .method("GET")
        .body(Body::empty())
        .unwrap()
}

async fn body_json(response: axum::http::Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ── /api/auth/providers tests ───────────────────────────────────────

#[tokio::test]
async fn test_providers_none_configured() {
    let app = test_app_no_oauth().await;
    let resp = app
        .oneshot(get_request("/api/auth/providers"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["github"], false);
    assert_eq!(body["google"], false);
    assert_eq!(body["local_mode"], true);
}

#[tokio::test]
async fn test_providers_github_only() {
    let app = test_app_with_github().await;
    let resp = app
        .oneshot(get_request("/api/auth/providers"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["github"], true);
    assert_eq!(body["google"], false);
}

#[tokio::test]
async fn test_providers_both_configured() {
    let app = test_app_with_both().await;
    let resp = app
        .oneshot(get_request("/api/auth/providers"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    assert_eq!(body["github"], true);
    assert_eq!(body["google"], true);
}

// ── GitHub auth start tests ─────────────────────────────────────────

#[tokio::test]
async fn test_github_auth_start_not_configured() {
    let app = test_app_no_oauth().await;
    let resp = app
        .oneshot(get_request("/api/auth/oauth/github"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let body = body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("not configured"));
}

#[tokio::test]
async fn test_github_auth_start_returns_url() {
    let app = test_app_with_github().await;
    let resp = app
        .oneshot(get_request("/api/auth/oauth/github"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let url = body["url"].as_str().unwrap();

    // Verify URL contains expected components
    assert!(url.starts_with("https://github.com/login/oauth/authorize"));
    assert!(url.contains("client_id=test-github-id"));
    assert!(url.contains("scope=user%3Aemail"));
    assert!(url.contains("state="));
    assert!(url.contains("code_challenge=")); // PKCE
    assert!(url.contains("code_challenge_method=S256")); // PKCE SHA256
}

#[tokio::test]
async fn test_github_auth_start_unique_states() {
    let app = test_app_with_github().await;

    let resp1 = app
        .clone()
        .oneshot(get_request("/api/auth/oauth/github"))
        .await
        .unwrap();
    let body1 = body_json(resp1).await;
    let url1 = body1["url"].as_str().unwrap().to_string();

    let resp2 = app
        .oneshot(get_request("/api/auth/oauth/github"))
        .await
        .unwrap();
    let body2 = body_json(resp2).await;
    let url2 = body2["url"].as_str().unwrap().to_string();

    // State parameters should be different for each request
    assert_ne!(url1, url2);
}

// ── Google auth start tests ─────────────────────────────────────────

#[tokio::test]
async fn test_google_auth_start_not_configured() {
    let app = test_app_no_oauth().await;
    let resp = app
        .oneshot(get_request("/api/auth/oauth/google"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_google_auth_start_returns_url() {
    let app = test_app_with_both().await;
    let resp = app
        .oneshot(get_request("/api/auth/oauth/google"))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = body_json(resp).await;
    let url = body["url"].as_str().unwrap();

    assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth"));
    assert!(url.contains("client_id=test-google-id"));
    assert!(url.contains("scope=")); // should contain openid, email, profile
    assert!(url.contains("state="));
    assert!(url.contains("code_challenge="));
    assert!(url.contains("code_challenge_method=S256"));
}
