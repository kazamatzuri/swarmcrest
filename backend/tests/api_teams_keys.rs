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

// ── Team tests ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_team() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/teams")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "Alpha Team"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "Alpha Team");
    assert!(body["id"].is_number());
}

#[tokio::test]
async fn test_list_teams() {
    let app = test_app().await;

    // Create two teams
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/teams")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "Team A"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/teams")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "Team B"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List teams
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/teams")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn test_get_team() {
    let app = test_app().await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/teams")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "My Team"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created = body_json(resp).await;
    let team_id = created["id"].as_i64().unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/teams/{}", team_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "My Team");
    assert_eq!(body["id"], team_id);
}

#[tokio::test]
async fn test_update_team() {
    let app = test_app().await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/teams")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "Old Name"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created = body_json(resp).await;
    let team_id = created["id"].as_i64().unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/teams/{}", team_id))
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "New Name"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "New Name");
}

#[tokio::test]
async fn test_delete_team() {
    let app = test_app().await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/teams")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "Doomed Team"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created = body_json(resp).await;
    let team_id = created["id"].as_i64().unwrap();

    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/teams/{}", team_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_create_team_version() {
    let app = test_app().await;

    // Create a team
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/teams")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "Duo Team"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let team = body_json(resp).await;
    let team_id = team["id"].as_i64().unwrap();

    // Create bot A
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/bots")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "Bot A"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let bot_a = body_json(resp).await;
    let bot_a_id = bot_a["id"].as_i64().unwrap();

    // Create version for bot A
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/bots/{}/versions", bot_a_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"code": "function Creature:main() end"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let ver_a = body_json(resp).await;
    let ver_a_id = ver_a["id"].as_i64().unwrap();

    // Create bot B
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/bots")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "Bot B"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let bot_b = body_json(resp).await;
    let bot_b_id = bot_b["id"].as_i64().unwrap();

    // Create version for bot B
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/bots/{}/versions", bot_b_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"code": "function Creature:main() end"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let ver_b = body_json(resp).await;
    let ver_b_id = ver_b["id"].as_i64().unwrap();

    // Create team version
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/teams/{}/versions", team_id))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"bot_version_a": ver_a_id, "bot_version_b": ver_b_id}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
}

// ── API key tests ────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_api_key() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/api-keys")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "CI Key"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let token = body["token"].as_str().unwrap();
    assert!(
        token.starts_with("sc_"),
        "token should start with sc_ prefix, got: {}",
        token
    );
    assert!(body["id"].is_number());
    assert_eq!(body["name"], "CI Key");
}

#[tokio::test]
async fn test_list_api_keys() {
    let app = test_app().await;

    // Create a key first
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/api-keys")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "List Test Key"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List keys
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/api-keys")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let keys = body.as_array().unwrap();
    assert!(!keys.is_empty());
    // Tokens should NOT be included in list responses
    for key in keys {
        assert!(key.get("token").is_none(), "token should not be in list response");
    }
}

#[tokio::test]
async fn test_delete_api_key() {
    let app = test_app().await;

    // Create a key
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/api-keys")
                .header("content-type", "application/json")
                .body(Body::from(json!({"name": "Delete Me"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let created = body_json(resp).await;
    let key_id = created["id"].as_i64().unwrap();

    // Delete it
    let resp = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/api-keys/{}", key_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

// ── Leaderboard tests ────────────────────────────────────────────────

#[tokio::test]
async fn test_leaderboard_1v1_empty() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/leaderboards/1v1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_leaderboard_ffa_empty() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/leaderboards/ffa")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.is_array());
}

#[tokio::test]
async fn test_leaderboard_2v2_empty() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/leaderboards/2v2")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body.is_array());
}

// ── Lua validation tests ─────────────────────────────────────────────

#[tokio::test]
async fn test_validate_lua_valid() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/validate-lua")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"code": "function Creature:main() end"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["valid"], true);
}

#[tokio::test]
async fn test_validate_lua_invalid() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/validate-lua")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"code": "this is not valid lua!!!"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["valid"], false);
    assert!(body.get("error").is_some(), "response should contain an error field");
}

// ── Misc tests ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_list_maps() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/maps")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_queue_status() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/queue/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_docs_lua_api() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/docs/lua-api")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_llms_txt() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/llms.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_list_matches_empty() {
    let app = test_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/matches")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
