// Application configuration, loaded from environment variables and CLI flags.

use std::path::PathBuf;

/// Application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Database URL (SQLite connection string).
    pub database_url: String,
    /// Port to bind the HTTP server to.
    pub port: u16,
    /// Directory containing map JSON files.
    pub maps_dir: PathBuf,
    /// Whether to run in local mode (no auth, no rate limiting).
    pub local_mode: bool,
    /// Directory containing pre-built frontend files to serve.
    /// When set, the backend serves static files from this path.
    pub static_dir: Option<PathBuf>,
    /// Number of parallel headless game workers.
    pub worker_count: usize,
    /// Interval in milliseconds between queue polls.
    pub queue_poll_ms: u64,
}

impl Config {
    /// Load configuration from environment variables and CLI arguments.
    ///
    /// Environment variables:
    /// - `DATABASE_URL` - SQLite connection string (default: `sqlite:infon.db?mode=rwc`)
    /// - `PORT` - HTTP server port (default: 3000)
    /// - `MAPS_DIR` - Path to maps directory (default: `../data/maps`)
    /// - `SWARMCREST_LOCAL_MODE` - Set to `true` to enable local mode
    /// - `STATIC_DIR` - Path to frontend dist directory for static file serving
    ///
    /// CLI flags:
    /// - `--local` - Enable local mode (same as `SWARMCREST_LOCAL_MODE=true`)
    /// - `--port <PORT>` - Override the port
    pub fn load() -> Self {
        let args: Vec<String> = std::env::args().collect();

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:infon.db?mode=rwc".to_string());

        // Port: CLI flag --port takes precedence, then env var, then default
        let port = Self::parse_cli_value(&args, "--port")
            .and_then(|v| v.parse().ok())
            .or_else(|| std::env::var("PORT").ok().and_then(|v| v.parse().ok()))
            .unwrap_or(3000);

        let maps_dir = std::env::var("MAPS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("../data/maps"));

        let local_mode = args.contains(&"--local".to_string())
            || std::env::var("SWARMCREST_LOCAL_MODE")
                .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                .unwrap_or(false);

        let static_dir = std::env::var("STATIC_DIR").ok().map(PathBuf::from);

        let worker_count = std::env::var("SWARMCREST_WORKER_COUNT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(4);

        let queue_poll_ms = std::env::var("SWARMCREST_QUEUE_POLL_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000);

        Config {
            database_url,
            port,
            maps_dir,
            local_mode,
            static_dir,
            worker_count,
            queue_poll_ms,
        }
    }

    /// Parse a CLI flag value like `--port 8080`.
    fn parse_cli_value(args: &[String], flag: &str) -> Option<String> {
        args.windows(2).find_map(|pair| {
            if pair[0] == flag {
                Some(pair[1].clone())
            } else {
                None
            }
        })
    }
}

/// Global flag indicating local mode is active.
/// This is set once at startup and read by auth extractors.
static LOCAL_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set the local mode flag (called once at startup).
pub fn set_local_mode(enabled: bool) {
    LOCAL_MODE.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

/// Check if local mode is active.
pub fn is_local_mode() -> bool {
    LOCAL_MODE.load(std::sync::atomic::Ordering::Relaxed)
}

/// The user ID used for the auto-created local user.
pub const LOCAL_USER_ID: i64 = 1;
pub const LOCAL_USERNAME: &str = "local";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_mode_flag() {
        set_local_mode(false);
        assert!(!is_local_mode());
        set_local_mode(true);
        assert!(is_local_mode());
        // Reset for other tests
        set_local_mode(false);
    }
}
