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

// Helper: create a tournament and return its JSON
async fn create_tournament(app: &axum::Router, name: &str) -> Value {
    let req = Request::builder()
        .method("POST")
        .uri("/api/tournaments")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"name": name}).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    body_json(resp).await
}

// Helper: create a bot and return its JSON
async fn create_bot(app: &axum::Router, name: &str) -> Value {
    let req = Request::builder()
        .method("POST")
        .uri("/api/bots")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"name": name}).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    body_json(resp).await
}

// Helper: create a bot version and return its JSON
async fn create_bot_version(app: &axum::Router, bot_id: i64, code: &str) -> Value {
    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/bots/{bot_id}/versions"))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"code": code}).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    body_json(resp).await
}

#[tokio::test]
async fn test_create_tournament() {
    let app = test_app().await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/tournaments")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"name": "Test Tourney"}).to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "Test Tourney");
    assert!(body["id"].is_number());
}

#[tokio::test]
async fn test_list_tournaments() {
    let app = test_app().await;
    create_tournament(&app, "Tourney A").await;
    create_tournament(&app, "Tourney B").await;

    let req = Request::builder()
        .uri("/api/tournaments")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let arr = body.as_array().expect("expected array");
    assert!(arr.len() >= 2);
}

#[tokio::test]
async fn test_get_tournament() {
    let app = test_app().await;
    let created = create_tournament(&app, "Get Me").await;
    let id = created["id"].as_i64().unwrap();

    let req = Request::builder()
        .uri(&format!("/api/tournaments/{id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "Get Me");
    assert_eq!(body["id"], id);
}

#[tokio::test]
async fn test_get_tournament_not_found() {
    let app = test_app().await;
    let req = Request::builder()
        .uri("/api/tournaments/99999")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_update_tournament() {
    let app = test_app().await;
    let created = create_tournament(&app, "Old Name").await;
    let id = created["id"].as_i64().unwrap();

    let req = Request::builder()
        .method("PUT")
        .uri(&format!("/api/tournaments/{id}"))
        .header("Content-Type", "application/json")
        .body(Body::from(json!({"name": "New Name"}).to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["name"], "New Name");
}

#[tokio::test]
async fn test_add_tournament_entry() {
    let app = test_app().await;
    let tournament = create_tournament(&app, "Entry Tourney").await;
    let tid = tournament["id"].as_i64().unwrap();

    let bot = create_bot(&app, "TestBot").await;
    let bot_id = bot["id"].as_i64().unwrap();
    let version = create_bot_version(&app, bot_id, "function Creature:main() end").await;
    let version_id = version["id"].as_i64().unwrap();

    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/tournaments/{tid}/entries"))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({"bot_version_id": version_id}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert_eq!(body["bot_version_id"], version_id);
}

#[tokio::test]
async fn test_list_tournament_entries() {
    let app = test_app().await;
    let tournament = create_tournament(&app, "List Entries Tourney").await;
    let tid = tournament["id"].as_i64().unwrap();

    let bot = create_bot(&app, "Bot1").await;
    let bot_id = bot["id"].as_i64().unwrap();
    let v1 = create_bot_version(&app, bot_id, "function Creature:main() end").await;
    let v1_id = v1["id"].as_i64().unwrap();

    let bot2 = create_bot(&app, "Bot2").await;
    let bot2_id = bot2["id"].as_i64().unwrap();
    let v2 = create_bot_version(&app, bot2_id, "function Creature:main() end").await;
    let v2_id = v2["id"].as_i64().unwrap();

    // Add two entries
    for vid in [v1_id, v2_id] {
        let req = Request::builder()
            .method("POST")
            .uri(&format!("/api/tournaments/{tid}/entries"))
            .header("Content-Type", "application/json")
            .body(Body::from(json!({"bot_version_id": vid}).to_string()))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = Request::builder()
        .uri(&format!("/api/tournaments/{tid}/entries"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let arr = body.as_array().expect("expected array");
    assert_eq!(arr.len(), 2);
}

#[tokio::test]
async fn test_remove_tournament_entry() {
    let app = test_app().await;
    let tournament = create_tournament(&app, "Remove Entry Tourney").await;
    let tid = tournament["id"].as_i64().unwrap();

    let bot = create_bot(&app, "RemoveBot").await;
    let bot_id = bot["id"].as_i64().unwrap();
    let version = create_bot_version(&app, bot_id, "function Creature:main() end").await;
    let version_id = version["id"].as_i64().unwrap();

    let req = Request::builder()
        .method("POST")
        .uri(&format!("/api/tournaments/{tid}/entries"))
        .header("Content-Type", "application/json")
        .body(Body::from(
            json!({"bot_version_id": version_id}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let entry = body_json(resp).await;
    let entry_id = entry["id"].as_i64().unwrap();

    let req = Request::builder()
        .method("DELETE")
        .uri(&format!("/api/tournaments/{tid}/entries/{entry_id}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_get_tournament_standings() {
    let app = test_app().await;
    let tournament = create_tournament(&app, "Standings Tourney").await;
    let tid = tournament["id"].as_i64().unwrap();

    let req = Request::builder()
        .uri(&format!("/api/tournaments/{tid}/standings"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_get_tournament_results() {
    let app = test_app().await;
    let tournament = create_tournament(&app, "Results Tourney").await;
    let tid = tournament["id"].as_i64().unwrap();

    let req = Request::builder()
        .uri(&format!("/api/tournaments/{tid}/results"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    let arr = body.as_array().expect("expected array");
    assert!(arr.is_empty());
}

#[tokio::test]
async fn test_get_tournament_matches() {
    let app = test_app().await;
    let tournament = create_tournament(&app, "Matches Tourney").await;
    let tid = tournament["id"].as_i64().unwrap();

    let req = Request::builder()
        .uri(&format!("/api/tournaments/{tid}/matches"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    // The response contains a "rounds" key with an empty array
    assert!(body["rounds"].is_array());
}
