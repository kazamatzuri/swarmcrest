#![allow(dead_code)]

mod api;
mod auth;
mod config;
mod db;
mod elo;
mod engine;
mod llms_txt;
mod metrics;
mod oauth;
mod queue;
mod rate_limit;
mod replay;
mod tournament;
mod worker_pool;

use axum::http::HeaderValue;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use config::Config;
use engine::server::GameServer;
use rate_limit::RateLimiter;
use worker_pool::WorkerPool;

async fn health_check() -> Json<Value> {
    Json(json!({ "status": "ok", "service": "swarmcrest-backend" }))
}

async fn metrics_handler(req: axum::http::Request<Body>) -> impl IntoResponse {
    // In local mode, allow unrestricted access
    if !crate::config::is_local_mode() {
        let metrics_token = std::env::var("METRICS_TOKEN").ok();
        if let Some(expected_token) = metrics_token {
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "));

            match auth_header {
                Some(token) if token == expected_token => {}
                _ => {
                    return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
                }
            }
        }
        // If METRICS_TOKEN is not set, metrics are publicly accessible (backwards compat)
        // Set METRICS_TOKEN to restrict access
    }

    let body = metrics::gather_metrics();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
        .into_response()
}

/// Axum middleware that records per-request metrics (count and duration).
async fn metrics_middleware(req: Request<Body>, next: Next) -> Response {
    let method = req.method().to_string();
    let path = metrics::normalize_path(req.uri().path());

    let start = std::time::Instant::now();
    let response = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();

    let status = response.status().as_u16().to_string();

    metrics::API_REQUESTS_TOTAL
        .with_label_values(&[&method, &path, &status])
        .inc();
    metrics::API_REQUEST_DURATION_SECONDS
        .with_label_values(&[&path])
        .observe(elapsed);

    response
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    metrics::register_metrics();

    let cfg = Config::load();

    // Set local mode flag globally so auth extractors can check it
    config::set_local_mode(cfg.local_mode);
    config::set_password_auth(cfg.password_auth_enabled);

    if cfg.local_mode {
        tracing::info!("==========================================================");
        tracing::info!("  Running in LOCAL MODE - no authentication required");
        tracing::info!("  Rate limiting is disabled");
        tracing::info!("==========================================================");
    }

    // DATABASE_URL supports both sqlite:// and postgres:// connection strings.
    // Examples:
    //   sqlite:swarmcrest.db?mode=rwc          (SQLite, default)
    //   sqlite::memory:                    (SQLite in-memory, for tests)
    //   postgres://user:pass@host/dbname   (PostgreSQL)
    // Install Any driver support for both SQLite and PostgreSQL.
    sqlx::any::install_default_drivers();

    let db = db::Database::new(&cfg.database_url)
        .await
        .expect("Failed to initialize database");
    let db = Arc::new(db);

    // In local mode, ensure a default user exists for auto-login
    if cfg.local_mode {
        ensure_local_user(&db).await;
    }

    // Clean up any matches/tournaments left in 'running' status from a prior server crash/restart
    match db.cleanup_orphaned_matches().await {
        Ok(0) => {}
        Ok(n) => tracing::info!("Marked {n} orphaned matches as abandoned"),
        Err(e) => tracing::error!("Failed to clean up orphaned matches: {e}"),
    }
    match db.cleanup_orphaned_tournaments().await {
        Ok(0) => {}
        Ok(n) => tracing::info!("Marked {n} orphaned tournaments as abandoned"),
        Err(e) => tracing::error!("Failed to clean up orphaned tournaments: {e}"),
    }

    // Clean up stale queue jobs from crashed workers
    match db.cleanup_stale_queue_jobs().await {
        Ok(0) => {}
        Ok(n) => tracing::info!("Reset {n} stale queue jobs back to pending"),
        Err(e) => tracing::error!("Failed to clean up stale queue jobs: {e}"),
    }

    // OAuth state for SSO providers (builds oauth2 clients at startup if configured)
    let oauth_state = Arc::new(oauth::OAuthState::new(&cfg, db.clone()));

    let game_server = Arc::new(GameServer::new());
    let rate_limiter = RateLimiter::new();
    let worker_pool = Arc::new(WorkerPool::new(cfg.worker_count));

    tracing::info!(
        "Worker pool: {} parallel headless game workers, polling every {}ms",
        cfg.worker_count,
        cfg.queue_poll_ms
    );

    // Generate a unique worker ID for this process
    let worker_id = format!("worker-{}", std::process::id());

    // Spawn background queue worker to process pending games
    crate::queue::spawn_queue_worker(
        db.clone(),
        worker_pool,
        cfg.maps_dir.clone(),
        cfg.queue_poll_ms,
        worker_id,
    );

    // Inject Arc<Database> into request extensions so auth extractors can
    // look up API tokens without needing access to AppState directly.
    let db_for_ext = db.clone();

    let cors = if cfg.local_mode {
        CorsLayer::permissive()
    } else {
        let origins_str = std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "https://swarmcrest.submerged-intelligence.de".to_string());
        let origins: Vec<HeaderValue> = origins_str
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any)
    };

    let mut app = Router::new()
        .route("/health", get(health_check))
        .route("/metrics", get(metrics_handler))
        // Auth routes (no auth required)
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/local", post(auth::local_login))
        .route("/api/auth/local-mode", get(auth::local_mode_status))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/profile", put(auth::update_profile))
        .with_state(db.clone())
        // OAuth SSO routes
        .route("/api/auth/providers", get(oauth::auth_providers))
        .route("/api/auth/oauth/github", get(oauth::github_auth_start))
        .route(
            "/api/auth/oauth/github/callback",
            get(oauth::github_auth_callback),
        )
        .route("/api/auth/oauth/google", get(oauth::google_auth_start))
        .route(
            "/api/auth/oauth/google/callback",
            get(oauth::google_auth_callback),
        )
        .with_state(oauth_state)
        .merge(api::router(
            db,
            game_server,
            rate_limiter,
            cfg.maps_dir.clone(),
        ))
        .layer(cors)
        .layer(axum::middleware::from_fn(metrics_middleware))
        .layer(axum::middleware::from_fn(
            move |mut req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| {
                let db = db_for_ext.clone();
                async move {
                    req.extensions_mut().insert(db);
                    next.run(req).await
                }
            },
        ));

    // Serve static frontend files if a static directory is configured
    if let Some(ref static_dir) = cfg.static_dir {
        if static_dir.exists() {
            tracing::info!("Serving static files from: {}", static_dir.display());
            // Serve static files, with SPA fallback to index.html for client-side routing
            let serve_dir = tower_http::services::ServeDir::new(static_dir).not_found_service(
                tower_http::services::ServeFile::new(static_dir.join("index.html")),
            );
            app = app.fallback_service(serve_dir);
        } else {
            tracing::warn!(
                "Static directory not found: {} - frontend will not be served",
                static_dir.display()
            );
        }
    }

    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to {}", addr));

    tracing::info!("SwarmCrest backend listening on port {}", cfg.port);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Failed to start server");
}

/// Ensure the default "local" user exists in the database for local mode.
async fn ensure_local_user(db: &db::Database) {
    match db.get_user_by_username(config::LOCAL_USERNAME).await {
        Ok(Some(_)) => {
            tracing::info!("Local user already exists");
        }
        Ok(None) => {
            // Create a local user with a placeholder password hash
            let password_hash = auth::hash_password("local-mode-password")
                .unwrap_or_else(|_| "not-a-real-hash".to_string());
            match db
                .create_user(
                    config::LOCAL_USERNAME,
                    "local@localhost",
                    &password_hash,
                    "Local Player",
                )
                .await
            {
                Ok(user) => {
                    tracing::info!("Created local user (id={})", user.id);
                }
                Err(e) => {
                    tracing::warn!("Failed to create local user: {e}");
                }
            }
        }
        Err(e) => {
            tracing::warn!("Failed to check for local user: {e}");
        }
    }
}
