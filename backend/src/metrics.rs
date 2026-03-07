// Prometheus metrics definitions for the SwarmCrest backend.

use lazy_static::lazy_static;
use prometheus::{
    Encoder, Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts,
    Registry, TextEncoder,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    // ── Gauges ───────────────────────────────────────────────────────

    /// Currently running games (0 or 1 for MVP, will scale later).
    pub static ref ACTIVE_GAMES: IntGauge =
        IntGauge::new("swarmcrest_active_games", "Currently running games").unwrap();

    /// Matches waiting in the queue to start.
    pub static ref GAME_QUEUE_DEPTH: IntGauge =
        IntGauge::new("swarmcrest_game_queue_depth", "Matches waiting to start").unwrap();

    /// Live WebSocket connections.
    pub static ref CONNECTED_WEBSOCKETS: IntGauge =
        IntGauge::new("swarmcrest_connected_websockets", "Live WebSocket connections").unwrap();

    /// Lua VMs currently executing (reserved for future pool usage).
    pub static ref LUA_VM_POOL_ACTIVE: IntGauge =
        IntGauge::new("swarmcrest_lua_vm_pool_active", "Lua VMs currently executing").unwrap();

    /// Headless game worker threads currently active.
    pub static ref HEADLESS_WORKERS_ACTIVE: IntGauge =
        IntGauge::new("swarmcrest_headless_workers_active", "Headless game workers currently active").unwrap();

    // ── Counters ─────────────────────────────────────────────────────

    /// Total games started, by format (1v1, ffa, 2v2).
    pub static ref GAMES_STARTED_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("swarmcrest_games_started_total", "Total games started"),
        &["format"],
    )
    .unwrap();

    /// Total games completed, by format.
    pub static ref GAMES_COMPLETED_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("swarmcrest_games_completed_total", "Total games completed"),
        &["format"],
    )
    .unwrap();

    /// Total games that errored, by format.
    pub static ref GAMES_ERRORED_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("swarmcrest_games_errored_total", "Total games that errored"),
        &["format"],
    )
    .unwrap();

    /// Total API requests, by method/endpoint/status.
    pub static ref API_REQUESTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("swarmcrest_api_requests_total", "Total API requests"),
        &["method", "endpoint", "status"],
    )
    .unwrap();

    /// Total WebSocket messages sent to clients.
    pub static ref WEBSOCKET_MESSAGES_SENT_TOTAL: IntCounter = IntCounter::new(
        "swarmcrest_websocket_messages_sent_total",
        "Total WebSocket messages sent",
    )
    .unwrap();

    /// Total new bot versions created.
    pub static ref BOT_SUBMISSIONS_TOTAL: IntCounter = IntCounter::new(
        "swarmcrest_bot_submissions_total",
        "New bot versions created",
    )
    .unwrap();

    /// Total bots that failed Lua validation.
    pub static ref BOT_VALIDATION_FAILURES_TOTAL: IntCounter = IntCounter::new(
        "swarmcrest_bot_validation_failures_total",
        "Bots that failed Lua validation",
    )
    .unwrap();

    /// Total creatures spawned, by creature type.
    pub static ref CREATURES_SPAWNED_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("swarmcrest_creatures_spawned_total", "Total creatures spawned"),
        &["creature_type"],
    )
    .unwrap();

    /// Total creatures killed, by creature type.
    pub static ref CREATURES_KILLED_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("swarmcrest_creatures_killed_total", "Total creatures killed"),
        &["creature_type"],
    )
    .unwrap();

    // ── Histograms ───────────────────────────────────────────────────

    /// Game duration in seconds, by format.
    pub static ref GAME_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new("swarmcrest_game_duration_seconds", "Game duration in seconds")
            .buckets(vec![10.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1200.0]),
        &["format"],
    )
    .unwrap();

    /// Per-tick processing time in milliseconds.
    pub static ref GAME_TICK_DURATION_MS: Histogram = Histogram::with_opts(
        HistogramOpts::new("swarmcrest_game_tick_duration_ms", "Per-tick processing time in ms")
            .buckets(vec![0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 25.0, 50.0, 100.0]),
    )
    .unwrap();

    /// API request duration in seconds, by endpoint.
    pub static ref API_REQUEST_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "swarmcrest_api_request_duration_seconds",
            "API request duration in seconds",
        )
        .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 5.0]),
        &["endpoint"],
    )
    .unwrap();
}

/// Register all metrics with the custom registry. Call once at startup.
pub fn register_metrics() {
    let collectors: Vec<Box<dyn prometheus::core::Collector>> = vec![
        Box::new(ACTIVE_GAMES.clone()),
        Box::new(GAME_QUEUE_DEPTH.clone()),
        Box::new(CONNECTED_WEBSOCKETS.clone()),
        Box::new(LUA_VM_POOL_ACTIVE.clone()),
        Box::new(HEADLESS_WORKERS_ACTIVE.clone()),
        Box::new(GAMES_STARTED_TOTAL.clone()),
        Box::new(GAMES_COMPLETED_TOTAL.clone()),
        Box::new(GAMES_ERRORED_TOTAL.clone()),
        Box::new(API_REQUESTS_TOTAL.clone()),
        Box::new(WEBSOCKET_MESSAGES_SENT_TOTAL.clone()),
        Box::new(BOT_SUBMISSIONS_TOTAL.clone()),
        Box::new(BOT_VALIDATION_FAILURES_TOTAL.clone()),
        Box::new(CREATURES_SPAWNED_TOTAL.clone()),
        Box::new(CREATURES_KILLED_TOTAL.clone()),
        Box::new(GAME_DURATION_SECONDS.clone()),
        Box::new(GAME_TICK_DURATION_MS.clone()),
        Box::new(API_REQUEST_DURATION_SECONDS.clone()),
    ];

    for c in collectors {
        REGISTRY.register(c).expect("failed to register metric");
    }
}

/// Serialize all registered metrics to the Prometheus text exposition format.
pub fn gather_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

/// Normalize a URL path for metric labels: replace numeric path segments with `:id`
/// to prevent cardinality explosion.
pub fn normalize_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if segment.parse::<i64>().is_ok() {
                ":id"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_no_ids() {
        assert_eq!(normalize_path("/api/bots"), "/api/bots");
        assert_eq!(normalize_path("/health"), "/health");
    }

    #[test]
    fn test_normalize_path_with_ids() {
        assert_eq!(normalize_path("/api/bots/42"), "/api/bots/:id");
        assert_eq!(
            normalize_path("/api/bots/42/versions/7"),
            "/api/bots/:id/versions/:id"
        );
    }

    #[test]
    fn test_normalize_path_preserves_non_numeric() {
        assert_eq!(normalize_path("/api/game/start"), "/api/game/start");
        assert_eq!(normalize_path("/ws/game"), "/ws/game");
    }

    #[test]
    fn test_gather_metrics_returns_string() {
        // Register and gather -- should not panic
        register_metrics();
        let output = gather_metrics();
        // Output should be empty or contain metric lines (no panic)
        assert!(output.is_empty() || output.contains("swarmcrest_"));
    }

    #[test]
    fn test_metric_increments() {
        // Just verify that incrementing metrics works without panicking
        ACTIVE_GAMES.set(1);
        assert_eq!(ACTIVE_GAMES.get(), 1);
        ACTIVE_GAMES.set(0);
        assert_eq!(ACTIVE_GAMES.get(), 0);

        GAME_QUEUE_DEPTH.set(3);
        assert_eq!(GAME_QUEUE_DEPTH.get(), 3);

        CONNECTED_WEBSOCKETS.inc();
        CONNECTED_WEBSOCKETS.dec();

        GAMES_STARTED_TOTAL.with_label_values(&["1v1"]).inc();
        GAMES_COMPLETED_TOTAL.with_label_values(&["ffa"]).inc();

        WEBSOCKET_MESSAGES_SENT_TOTAL.inc();
        BOT_SUBMISSIONS_TOTAL.inc();
        BOT_VALIDATION_FAILURES_TOTAL.inc();

        CREATURES_SPAWNED_TOTAL
            .with_label_values(&["small"])
            .inc();
        CREATURES_KILLED_TOTAL.with_label_values(&["big"]).inc();

        GAME_TICK_DURATION_MS.observe(1.5);
        GAME_DURATION_SECONDS
            .with_label_values(&["1v1"])
            .observe(300.0);
        API_REQUEST_DURATION_SECONDS
            .with_label_values(&["/api/bots"])
            .observe(0.05);

        API_REQUESTS_TOTAL
            .with_label_values(&["GET", "/api/bots", "200"])
            .inc();
    }
}
