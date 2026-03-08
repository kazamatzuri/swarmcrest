// Integration tests for match and challenge API endpoints.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

async fn test_app() -> axum::Router {
    sqlx::any::install_default_drivers();
    let db = swarmcrest_backend::db::Database::new("sqlite::memory:")
        .await
        .unwrap();
    let db = Arc::new(db);
    let game_server = Arc::new(swarmcrest_backend::engine::server::GameServer::new());
    let rate_limiter = swarmcrest_backend::rate_limit::RateLimiter::new();
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
    let maps_dir = std::path::PathBuf::from("../data/maps");
    swarmcrest_backend::api::router(db, game_server, rate_limiter, maps_dir)
}

async fn body_json(response: axum::http::Response<Body>) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Helper: create a bot and version, return (bot_id, version_id).
async fn create_bot_with_version(app: &axum::Router) -> (i64, i64) {
    let req = Request::builder()
        .uri("/api/bots")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({"name": "TestBot"})).unwrap(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let bot: Value = body_json(resp).await;
    let bot_id = bot["id"].as_i64().unwrap();

    let req = Request::builder()
        .uri(format!("/api/bots/{}/versions", bot_id))
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({"code": "function Creature:main() end"})).unwrap(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let version: Value = body_json(resp).await;
    let version_id = version["id"].as_i64().unwrap();

    (bot_id, version_id)
}

// ── Test: list matches (empty) ───────────────────────────────────────

#[tokio::test]
async fn test_list_matches_empty() {
    let app = test_app().await;
    let req = Request::builder()
        .uri("/api/matches")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.is_array(), "Expected array, got: {body}");
    assert_eq!(body.as_array().unwrap().len(), 0);
}

// ── Test: list my matches (empty) ────────────────────────────────────

#[tokio::test]
async fn test_list_my_matches_empty() {
    let app = test_app().await;
    let req = Request::builder()
        .uri("/api/matches/mine")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.is_array(), "Expected array, got: {body}");
}

// ── Test: get match not found ────────────────────────────────────────

#[tokio::test]
async fn test_get_match_not_found() {
    let app = test_app().await;
    let req = Request::builder()
        .uri("/api/matches/99999")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Test: get match replay not found ─────────────────────────────────

#[tokio::test]
async fn test_get_match_replay_not_found() {
    let app = test_app().await;
    let req = Request::builder()
        .uri("/api/matches/99999/replay")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ── Test: create headless challenge ──────────────────────────────────

#[tokio::test]
async fn test_create_challenge_headless() {
    let app = test_app().await;
    let (_bot_a, version_a) = create_bot_with_version(&app).await;
    let (_bot_b, version_b) = create_bot_with_version(&app).await;

    let req = Request::builder()
        .uri("/api/matches/challenge")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "bot_version_id": version_a,
                "opponent_bot_version_id": version_b,
                "headless": true
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    assert!(
        status == StatusCode::OK || status == StatusCode::CREATED,
        "Expected 200 or 201, got {status}"
    );
    let body = body_json(resp).await;
    assert!(
        body["match_id"].is_number(),
        "Expected match_id in response: {body}"
    );
    assert_eq!(body["status"], "queued");
}

// ── Test: challenge bot vs itself ────────────────────────────────────

#[tokio::test]
async fn test_create_challenge_same_bot() {
    let app = test_app().await;
    let (_bot, version) = create_bot_with_version(&app).await;

    let req = Request::builder()
        .uri("/api/matches/challenge")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "bot_version_id": version,
                "opponent_bot_version_id": version,
                "headless": true
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    assert!(
        status == StatusCode::OK || status == StatusCode::CREATED,
        "Expected 200 or 201, got {status}"
    );
    let body = body_json(resp).await;
    assert!(body["match_id"].is_number(), "Expected match_id: {body}");
}

// ── Test: challenge with invalid version ids ─────────────────────────

#[tokio::test]
async fn test_create_challenge_invalid_version() {
    let app = test_app().await;

    let req = Request::builder()
        .uri("/api/matches/challenge")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "bot_version_id": 99999,
                "opponent_bot_version_id": 99998,
                "headless": true
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::BAD_REQUEST,
        "Expected 404 or 400, got {status}"
    );
}

// ── Test: get match after challenge ──────────────────────────────────

#[tokio::test]
async fn test_get_match_after_challenge() {
    let app = test_app().await;
    let (_bot_a, version_a) = create_bot_with_version(&app).await;
    let (_bot_b, version_b) = create_bot_with_version(&app).await;

    // Create a headless challenge
    let req = Request::builder()
        .uri("/api/matches/challenge")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "bot_version_id": version_a,
                "opponent_bot_version_id": version_b,
                "headless": true
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let body = body_json(resp).await;
    let match_id = body["match_id"].as_i64().unwrap();

    // GET the match by id
    let req = Request::builder()
        .uri(format!("/api/matches/{match_id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["match"].is_object(), "Expected match object: {body}");
    assert!(
        body["participants"].is_array(),
        "Expected participants array: {body}"
    );
}

// ── Test: list matches after challenge ───────────────────────────────

#[tokio::test]
async fn test_list_matches_after_challenge() {
    let app = test_app().await;
    let (_bot_a, version_a) = create_bot_with_version(&app).await;
    let (_bot_b, version_b) = create_bot_with_version(&app).await;

    // Create a headless challenge
    let req = Request::builder()
        .uri("/api/matches/challenge")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "bot_version_id": version_a,
                "opponent_bot_version_id": version_b,
                "headless": true
            }))
            .unwrap(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let challenge_body = body_json(resp).await;
    let match_id = challenge_body["match_id"].as_i64().unwrap();

    // List all matches
    let req = Request::builder()
        .uri("/api/matches")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let matches = body.as_array().expect("Expected array of matches");
    assert!(
        matches.iter().any(|m| m["id"].as_i64() == Some(match_id)),
        "Expected match {match_id} in list: {body}"
    );
}

// ── Test: list matches with status filter ────────────────────────────

#[tokio::test]
async fn test_list_matches_with_filters() {
    let app = test_app().await;

    // Query with a status filter on an empty DB — should return 200
    let req = Request::builder()
        .uri("/api/matches?status=pending")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.is_array(), "Expected array, got: {body}");
}
