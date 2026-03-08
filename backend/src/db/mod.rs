// Database access layer using sqlx's Any driver (supports SQLite and PostgreSQL).

use serde::{Deserialize, Serialize, Serializer};
use sqlx::any::{AnyPoolOptions, AnyQueryResult};
use sqlx::AnyPool;

/// Serialize an i32 as a boolean (0 = false, non-zero = true).
/// Used for columns stored as INTEGER in SQLite but logically boolean.
fn serialize_int_as_bool<S>(val: &i32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_bool(*val != 0)
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OAuthAccount {
    pub id: i64,
    pub user_id: i64,
    pub provider: String,
    pub provider_user_id: String,
    pub provider_username: Option<String>,
    pub provider_email: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Bot {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub owner_id: Option<i64>,
    pub visibility: String,
    pub active_version_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// Enriched bot info for the library view, includes version stats.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BotSummary {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub owner_id: Option<i64>,
    pub owner_username: Option<String>,
    pub visibility: String,
    pub created_at: String,
    pub updated_at: String,
    pub version_count: i32,
    pub latest_version: Option<i32>,
    pub latest_elo_1v1: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BotVersion {
    pub id: i64,
    pub bot_id: i64,
    pub version: i32,
    pub code: String,
    pub api_type: String,
    /// Stored as INTEGER (0/1) for cross-database compatibility with the Any driver.
    /// Serialized as boolean for API consumers.
    #[serde(serialize_with = "serialize_int_as_bool")]
    pub is_archived: i32,
    /// Whether this version failed to load in a game (e.g. Lua syntax error).
    #[serde(serialize_with = "serialize_int_as_bool")]
    pub is_faulty: i32,
    pub elo_rating: i32,
    pub elo_1v1: i32,
    pub elo_peak: i32,
    pub games_played: i32,
    pub wins: i32,
    pub losses: i32,
    pub draws: i32,
    pub ffa_placement_points: i32,
    pub ffa_games: i32,
    pub creatures_spawned: i32,
    pub creatures_killed: i32,
    pub creatures_lost: i32,
    pub total_score: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Match {
    pub id: i64,
    pub format: String,
    pub map: String,
    pub status: String,
    pub winner_bot_version_id: Option<i64>,
    pub created_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MatchParticipant {
    pub id: i64,
    pub match_id: i64,
    pub bot_version_id: i64,
    pub player_slot: i32,
    pub final_score: i32,
    pub placement: Option<i32>,
    pub elo_before: Option<i32>,
    pub elo_after: Option<i32>,
    pub creatures_spawned: i32,
    pub creatures_killed: i32,
    pub creatures_lost: i32,
    pub bot_name: Option<String>,
    pub owner_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tournament {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub map: String,
    pub config: String,
    pub format: String,
    pub current_round: i32,
    pub total_rounds: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentStanding {
    pub bot_version_id: i64,
    pub bot_name: String,
    pub total_score: i64,
    pub matches_played: i32,
    pub wins: i32,
    pub losses: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TournamentEntry {
    pub id: i64,
    pub tournament_id: i64,
    pub bot_version_id: i64,
    pub slot_name: String,
    pub bot_name: Option<String>,
    pub version: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TournamentResult {
    pub id: i64,
    pub tournament_id: i64,
    pub player_slot: i32,
    pub bot_version_id: i64,
    pub final_score: i32,
    pub creatures_spawned: i32,
    pub creatures_killed: i32,
    pub creatures_lost: i32,
    pub finished_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TournamentMatch {
    pub id: i64,
    pub tournament_id: i64,
    pub match_id: i64,
    pub round: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TournamentMatchDetail {
    pub match_id: i64,
    pub round: i32,
    pub status: String,
    pub winner_bot_version_id: Option<i64>,
    pub finished_at: Option<String>,
    pub participants: Vec<TournamentMatchParticipantInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TournamentMatchParticipantInfo {
    pub match_id: i64,
    pub bot_version_id: i64,
    pub player_slot: i32,
    pub final_score: i32,
    pub bot_name: Option<String>,
    pub owner_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LeaderboardEntry {
    pub rank: i64,
    pub bot_version_id: i64,
    pub bot_name: String,
    pub version: i32,
    pub owner_username: String,
    pub rating: i32,
    pub games_played: i32,
    pub wins: i32,
    pub losses: i32,
    pub win_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiToken {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub token_hash: String,
    pub scopes: String,
    pub last_used_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Replay {
    pub id: i64,
    pub match_id: i64,
    pub data: Vec<u8>,
    pub tick_count: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Team {
    pub id: i64,
    pub owner_id: i64,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamVersion {
    pub id: i64,
    pub team_id: i64,
    pub version: i32,
    pub bot_version_a: i64,
    pub bot_version_b: i64,
    pub elo_rating: i32,
    pub games_played: i32,
    pub wins: i32,
    pub losses: i32,
    pub draws: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: i64,
    pub user_id: i64,
    #[serde(rename = "type")]
    #[sqlx(rename = "type")]
    pub notification_type: String,
    pub title: String,
    pub message: String,
    pub data: Option<String>,
    #[serde(serialize_with = "serialize_int_as_bool")]
    pub read: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Feedback {
    pub id: i64,
    pub user_id: Option<i64>,
    pub category: String,
    pub description: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct QueueJob {
    pub id: i64,
    pub match_id: i64,
    pub status: String,
    pub worker_id: Option<String>,
    pub map: Option<String>,
    pub map_params: Option<String>,
    pub priority: i32,
    pub attempts: i32,
    pub max_attempts: i32,
    pub error_message: Option<String>,
    pub claimed_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameQueueStatus {
    pub pending: i64,
    pub claimed: i64,
    pub completed: i64,
    pub failed: i64,
    pub total: i64,
}

pub struct Database {
    pool: AnyPool,
    is_postgres: bool,
}

impl Database {
    /// Execute a raw SQL statement, returning the query result.
    /// This helper exists to provide type information for the Any driver.
    async fn exec(&self, sql: &str) -> Result<AnyQueryResult, sqlx::Error> {
        sqlx::query(sql).execute(&self.pool).await
    }

    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let is_postgres =
            database_url.starts_with("postgres://") || database_url.starts_with("postgresql://");
        // For SQLite in-memory databases, limit to 1 connection so all
        // queries share the same in-memory database.
        let is_memory = database_url.contains(":memory:");
        let max_conn = if is_memory { 1 } else { 5 };
        let pool = AnyPoolOptions::new()
            .max_connections(max_conn)
            .connect(database_url)
            .await?;
        let db = Self { pool, is_postgres };
        db.run_migrations().await?;
        Ok(db)
    }

    async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        if self.is_postgres {
            self.run_migrations_postgres().await
        } else {
            self.run_migrations_sqlite().await
        }
    }

    async fn run_migrations_postgres(&self) -> Result<(), sqlx::Error> {
        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id BIGSERIAL PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT,
                display_name TEXT,
                avatar_url TEXT,
                bio TEXT,
                role TEXT NOT NULL DEFAULT 'user',
                created_at TEXT NOT NULL DEFAULT (now()::text),
                updated_at TEXT NOT NULL DEFAULT (now()::text)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS bots (
                id BIGSERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                owner_id BIGINT REFERENCES users(id),
                visibility TEXT NOT NULL DEFAULT 'public',
                active_version_id BIGINT,
                created_at TEXT NOT NULL DEFAULT (now()::text),
                updated_at TEXT NOT NULL DEFAULT (now()::text)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS bot_versions (
                id BIGSERIAL PRIMARY KEY,
                bot_id BIGINT NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
                version INTEGER NOT NULL,
                code TEXT NOT NULL,
                api_type TEXT NOT NULL DEFAULT 'oo',
                is_archived INTEGER NOT NULL DEFAULT 0,
                is_faulty INTEGER NOT NULL DEFAULT 0,
                elo_rating INTEGER NOT NULL DEFAULT 1500,
                elo_1v1 INTEGER NOT NULL DEFAULT 1500,
                elo_peak INTEGER NOT NULL DEFAULT 1500,
                games_played INTEGER NOT NULL DEFAULT 0,
                wins INTEGER NOT NULL DEFAULT 0,
                losses INTEGER NOT NULL DEFAULT 0,
                draws INTEGER NOT NULL DEFAULT 0,
                ffa_placement_points INTEGER NOT NULL DEFAULT 0,
                ffa_games INTEGER NOT NULL DEFAULT 0,
                creatures_spawned INTEGER NOT NULL DEFAULT 0,
                creatures_killed INTEGER NOT NULL DEFAULT 0,
                creatures_lost INTEGER NOT NULL DEFAULT 0,
                total_score BIGINT NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (now()::text),
                UNIQUE(bot_id, version)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS matches (
                id BIGSERIAL PRIMARY KEY,
                format TEXT NOT NULL DEFAULT '1v1',
                map TEXT NOT NULL DEFAULT 'random',
                status TEXT NOT NULL DEFAULT 'pending',
                winner_bot_version_id BIGINT,
                created_at TEXT NOT NULL DEFAULT (now()::text),
                finished_at TEXT
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS match_participants (
                id BIGSERIAL PRIMARY KEY,
                match_id BIGINT NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
                bot_version_id BIGINT NOT NULL REFERENCES bot_versions(id),
                player_slot INTEGER NOT NULL,
                final_score INTEGER NOT NULL DEFAULT 0,
                placement INTEGER,
                elo_before INTEGER,
                elo_after INTEGER,
                creatures_spawned INTEGER NOT NULL DEFAULT 0,
                creatures_killed INTEGER NOT NULL DEFAULT 0,
                creatures_lost INTEGER NOT NULL DEFAULT 0
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS tournaments (
                id BIGSERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'created',
                map TEXT NOT NULL DEFAULT 'default',
                config TEXT NOT NULL DEFAULT '{}',
                format TEXT NOT NULL DEFAULT 'round_robin',
                current_round INTEGER NOT NULL DEFAULT 0,
                total_rounds INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (now()::text)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS tournament_entries (
                id BIGSERIAL PRIMARY KEY,
                tournament_id BIGINT NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
                bot_version_id BIGINT NOT NULL REFERENCES bot_versions(id),
                slot_name TEXT NOT NULL DEFAULT ''
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS tournament_results (
                id BIGSERIAL PRIMARY KEY,
                tournament_id BIGINT NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
                player_slot INTEGER NOT NULL,
                bot_version_id BIGINT NOT NULL REFERENCES bot_versions(id),
                final_score INTEGER NOT NULL DEFAULT 0,
                creatures_spawned INTEGER NOT NULL DEFAULT 0,
                creatures_killed INTEGER NOT NULL DEFAULT 0,
                creatures_lost INTEGER NOT NULL DEFAULT 0,
                finished_at TEXT NOT NULL DEFAULT (now()::text)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS tournament_matches (
                id BIGSERIAL PRIMARY KEY,
                tournament_id BIGINT NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
                match_id BIGINT NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
                round INTEGER NOT NULL DEFAULT 1
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS teams (
                id BIGSERIAL PRIMARY KEY,
                owner_id BIGINT NOT NULL REFERENCES users(id),
                name TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (now()::text)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS team_versions (
                id BIGSERIAL PRIMARY KEY,
                team_id BIGINT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
                version INTEGER NOT NULL,
                bot_version_a BIGINT NOT NULL REFERENCES bot_versions(id),
                bot_version_b BIGINT NOT NULL REFERENCES bot_versions(id),
                elo_rating INTEGER NOT NULL DEFAULT 1500,
                games_played INTEGER NOT NULL DEFAULT 0,
                wins INTEGER NOT NULL DEFAULT 0,
                losses INTEGER NOT NULL DEFAULT 0,
                draws INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (now()::text),
                UNIQUE(team_id, version)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS replays (
                id BIGSERIAL PRIMARY KEY,
                match_id BIGINT NOT NULL UNIQUE REFERENCES matches(id) ON DELETE CASCADE,
                data BYTEA NOT NULL,
                tick_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (now()::text)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS api_tokens (
                id BIGSERIAL PRIMARY KEY,
                user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                token_hash TEXT NOT NULL,
                scopes TEXT NOT NULL DEFAULT 'bots:read,matches:read,leaderboard:read',
                last_used_at TEXT,
                created_at TEXT NOT NULL DEFAULT (now()::text)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS notifications (
                id BIGSERIAL PRIMARY KEY,
                user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                type TEXT NOT NULL DEFAULT 'info',
                title TEXT NOT NULL,
                message TEXT NOT NULL DEFAULT '',
                data TEXT,
                read INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (now()::text)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS feedback (
                id BIGSERIAL PRIMARY KEY,
                user_id BIGINT REFERENCES users(id),
                category TEXT NOT NULL DEFAULT 'general',
                description TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (now()::text)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS game_queue (
                id BIGSERIAL PRIMARY KEY,
                match_id BIGINT NOT NULL REFERENCES matches(id),
                status TEXT NOT NULL DEFAULT 'pending',
                worker_id TEXT,
                map TEXT,
                map_params TEXT,
                headless BOOLEAN NOT NULL DEFAULT TRUE,
                priority INTEGER NOT NULL DEFAULT 0,
                attempts INTEGER NOT NULL DEFAULT 0,
                max_attempts INTEGER NOT NULL DEFAULT 3,
                error_message TEXT,
                claimed_at TEXT,
                completed_at TEXT,
                created_at TEXT NOT NULL DEFAULT (now()::text)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE INDEX IF NOT EXISTS idx_game_queue_pending
            ON game_queue(status, priority DESC, created_at)
        "#,
        )
        .await?;

        // Add map_params column to existing game_queue tables
        let _ = self
            .exec("ALTER TABLE game_queue ADD COLUMN map_params TEXT")
            .await;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_accounts (
                id BIGSERIAL PRIMARY KEY,
                user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                provider TEXT NOT NULL,
                provider_user_id TEXT NOT NULL,
                provider_username TEXT,
                provider_email TEXT,
                created_at TEXT NOT NULL DEFAULT (now()::text),
                UNIQUE(provider, provider_user_id)
            )
        "#,
        )
        .await?;

        Ok(())
    }

    async fn run_migrations_sqlite(&self) -> Result<(), sqlx::Error> {
        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT,
                display_name TEXT,
                avatar_url TEXT,
                bio TEXT,
                role TEXT NOT NULL DEFAULT 'user',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS bots (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                owner_id INTEGER REFERENCES users(id),
                visibility TEXT NOT NULL DEFAULT 'public',
                active_version_id INTEGER,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .await?;

        // Add columns to existing bots table if missing
        let _ = self
            .exec("ALTER TABLE bots ADD COLUMN owner_id INTEGER REFERENCES users(id)")
            .await;
        let _ = self
            .exec("ALTER TABLE bots ADD COLUMN visibility TEXT NOT NULL DEFAULT 'public'")
            .await;
        let _ = self
            .exec("ALTER TABLE bots ADD COLUMN active_version_id INTEGER")
            .await;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS bot_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                bot_id INTEGER NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
                version INTEGER NOT NULL,
                code TEXT NOT NULL,
                api_type TEXT NOT NULL DEFAULT 'oo',
                is_archived INTEGER NOT NULL DEFAULT 0,
                is_faulty INTEGER NOT NULL DEFAULT 0,
                elo_rating INTEGER NOT NULL DEFAULT 1500,
                elo_1v1 INTEGER NOT NULL DEFAULT 1500,
                elo_peak INTEGER NOT NULL DEFAULT 1500,
                games_played INTEGER NOT NULL DEFAULT 0,
                wins INTEGER NOT NULL DEFAULT 0,
                losses INTEGER NOT NULL DEFAULT 0,
                draws INTEGER NOT NULL DEFAULT 0,
                ffa_placement_points INTEGER NOT NULL DEFAULT 0,
                ffa_games INTEGER NOT NULL DEFAULT 0,
                creatures_spawned INTEGER NOT NULL DEFAULT 0,
                creatures_killed INTEGER NOT NULL DEFAULT 0,
                creatures_lost INTEGER NOT NULL DEFAULT 0,
                total_score INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(bot_id, version)
            )
        "#,
        )
        .await?;

        // Add new columns to existing bot_versions if missing
        for col in &[
            "is_archived INTEGER NOT NULL DEFAULT 0",
            "is_faulty INTEGER NOT NULL DEFAULT 0",
            "elo_rating INTEGER NOT NULL DEFAULT 1500",
            "elo_1v1 INTEGER NOT NULL DEFAULT 1500",
            "elo_peak INTEGER NOT NULL DEFAULT 1500",
            "games_played INTEGER NOT NULL DEFAULT 0",
            "wins INTEGER NOT NULL DEFAULT 0",
            "losses INTEGER NOT NULL DEFAULT 0",
            "draws INTEGER NOT NULL DEFAULT 0",
            "ffa_placement_points INTEGER NOT NULL DEFAULT 0",
            "ffa_games INTEGER NOT NULL DEFAULT 0",
            "creatures_spawned INTEGER NOT NULL DEFAULT 0",
            "creatures_killed INTEGER NOT NULL DEFAULT 0",
            "creatures_lost INTEGER NOT NULL DEFAULT 0",
            "total_score INTEGER NOT NULL DEFAULT 0",
        ] {
            let _ = self
                .exec(&format!("ALTER TABLE bot_versions ADD COLUMN {col}"))
                .await;
        }

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS matches (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                format TEXT NOT NULL DEFAULT '1v1',
                map TEXT NOT NULL DEFAULT 'random',
                status TEXT NOT NULL DEFAULT 'pending',
                winner_bot_version_id INTEGER,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                finished_at TEXT
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS match_participants (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                match_id INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
                bot_version_id INTEGER NOT NULL REFERENCES bot_versions(id),
                player_slot INTEGER NOT NULL,
                final_score INTEGER NOT NULL DEFAULT 0,
                placement INTEGER,
                elo_before INTEGER,
                elo_after INTEGER,
                creatures_spawned INTEGER NOT NULL DEFAULT 0,
                creatures_killed INTEGER NOT NULL DEFAULT 0,
                creatures_lost INTEGER NOT NULL DEFAULT 0
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS tournaments (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'created',
                map TEXT NOT NULL DEFAULT 'default',
                config TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .await?;

        // Add new columns to existing tournaments table if missing
        for col in &[
            "format TEXT NOT NULL DEFAULT 'round_robin'",
            "current_round INTEGER NOT NULL DEFAULT 0",
            "total_rounds INTEGER NOT NULL DEFAULT 1",
        ] {
            let _ = self
                .exec(&format!("ALTER TABLE tournaments ADD COLUMN {col}"))
                .await;
        }

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS tournament_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tournament_id INTEGER NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
                bot_version_id INTEGER NOT NULL REFERENCES bot_versions(id),
                slot_name TEXT NOT NULL DEFAULT ''
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS tournament_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tournament_id INTEGER NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
                player_slot INTEGER NOT NULL,
                bot_version_id INTEGER NOT NULL REFERENCES bot_versions(id),
                final_score INTEGER NOT NULL DEFAULT 0,
                creatures_spawned INTEGER NOT NULL DEFAULT 0,
                creatures_killed INTEGER NOT NULL DEFAULT 0,
                creatures_lost INTEGER NOT NULL DEFAULT 0,
                finished_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS tournament_matches (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tournament_id INTEGER NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
                match_id INTEGER NOT NULL REFERENCES matches(id) ON DELETE CASCADE,
                round INTEGER NOT NULL DEFAULT 1
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS teams (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                owner_id INTEGER NOT NULL REFERENCES users(id),
                name TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS team_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
                version INTEGER NOT NULL,
                bot_version_a INTEGER NOT NULL REFERENCES bot_versions(id),
                bot_version_b INTEGER NOT NULL REFERENCES bot_versions(id),
                elo_rating INTEGER NOT NULL DEFAULT 1500,
                games_played INTEGER NOT NULL DEFAULT 0,
                wins INTEGER NOT NULL DEFAULT 0,
                losses INTEGER NOT NULL DEFAULT 0,
                draws INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(team_id, version)
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS replays (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                match_id INTEGER NOT NULL UNIQUE REFERENCES matches(id) ON DELETE CASCADE,
                data BLOB NOT NULL,
                tick_count INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS api_tokens (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                name TEXT NOT NULL,
                token_hash TEXT NOT NULL,
                scopes TEXT NOT NULL DEFAULT 'bots:read,matches:read,leaderboard:read',
                last_used_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .await?;

        // Notifications table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS notifications (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                type TEXT NOT NULL DEFAULT 'info',
                title TEXT NOT NULL,
                message TEXT NOT NULL DEFAULT '',
                data TEXT,
                read INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        // Feedback table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS feedback (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER REFERENCES users(id),
                category TEXT NOT NULL DEFAULT 'general',
                description TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .execute(&self.pool)
        .await?;

        // Game queue table
        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS game_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                match_id INTEGER NOT NULL REFERENCES matches(id),
                status TEXT NOT NULL DEFAULT 'pending',
                worker_id TEXT,
                map TEXT,
                map_params TEXT,
                headless INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 0,
                attempts INTEGER NOT NULL DEFAULT 0,
                max_attempts INTEGER NOT NULL DEFAULT 3,
                error_message TEXT,
                claimed_at TEXT,
                completed_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
        "#,
        )
        .await?;

        self.exec(
            r#"
            CREATE INDEX IF NOT EXISTS idx_game_queue_pending
            ON game_queue(status, priority DESC, created_at)
        "#,
        )
        .await?;

        // Add map_params column to existing game_queue tables
        let _ = self
            .exec("ALTER TABLE game_queue ADD COLUMN map_params TEXT")
            .await;

        self.exec(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_accounts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                provider TEXT NOT NULL,
                provider_user_id TEXT NOT NULL,
                provider_username TEXT,
                provider_email TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(provider, provider_user_id)
            )
        "#,
        )
        .await?;

        Ok(())
    }

    /// Returns the SQL expression for the current timestamp as text,
    /// appropriate for the connected database backend.
    fn now_expr(&self) -> &'static str {
        if self.is_postgres {
            "now()::text"
        } else {
            "datetime('now')"
        }
    }

    // ── User CRUD ─────────────────────────────────────────────────────

    pub async fn create_user(
        &self,
        username: &str,
        email: &str,
        password_hash: &str,
        display_name: &str,
    ) -> Result<User, sqlx::Error> {
        let row = sqlx::query_as::<_, User>(
            "INSERT INTO users (username, email, password_hash, display_name) VALUES ($1, $2, $3, $4) RETURNING id, username, email, password_hash, display_name, avatar_url, bio, role, created_at, updated_at",
        )
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .bind(display_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn get_user(&self, id: i64) -> Result<Option<User>, sqlx::Error> {
        let row = sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, display_name, avatar_url, bio, role, created_at, updated_at FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error> {
        let row = sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, display_name, avatar_url, bio, role, created_at, updated_at FROM users WHERE username = $1",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_user(
        &self,
        id: i64,
        display_name: Option<&str>,
        bio: Option<&str>,
    ) -> Result<Option<User>, sqlx::Error> {
        let sql = format!(
            "UPDATE users SET display_name = COALESCE($1, display_name), bio = COALESCE($2, bio), updated_at = {} WHERE id = $3",
            self.now_expr()
        );
        let result: AnyQueryResult = sqlx::query(&sql)
            .bind(display_name)
            .bind(bio)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }
        self.get_user(id).await
    }

    // ── OAuth account helpers ────────────────────────────────────────

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, sqlx::Error> {
        let row = sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, display_name, avatar_url, bio, role, created_at, updated_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn find_oauth_account(
        &self,
        provider: &str,
        provider_user_id: &str,
    ) -> Result<Option<OAuthAccount>, sqlx::Error> {
        let row = sqlx::query_as::<_, OAuthAccount>(
            "SELECT id, user_id, provider, provider_user_id, provider_username, provider_email, created_at FROM oauth_accounts WHERE provider = $1 AND provider_user_id = $2",
        )
        .bind(provider)
        .bind(provider_user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn create_oauth_account(
        &self,
        user_id: i64,
        provider: &str,
        provider_user_id: &str,
        provider_username: Option<&str>,
        provider_email: Option<&str>,
    ) -> Result<OAuthAccount, sqlx::Error> {
        let row = sqlx::query_as::<_, OAuthAccount>(
            "INSERT INTO oauth_accounts (user_id, provider, provider_user_id, provider_username, provider_email) VALUES ($1, $2, $3, $4, $5) RETURNING id, user_id, provider, provider_user_id, provider_username, provider_email, created_at",
        )
        .bind(user_id)
        .bind(provider)
        .bind(provider_user_id)
        .bind(provider_username)
        .bind(provider_email)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn create_user_oauth(
        &self,
        username: &str,
        email: &str,
        display_name: &str,
        avatar_url: Option<&str>,
    ) -> Result<User, sqlx::Error> {
        let row = sqlx::query_as::<_, User>(
            "INSERT INTO users (username, email, password_hash, display_name, avatar_url) VALUES ($1, $2, NULL, $3, $4) RETURNING id, username, email, password_hash, display_name, avatar_url, bio, role, created_at, updated_at",
        )
        .bind(username)
        .bind(email)
        .bind(display_name)
        .bind(avatar_url)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_user_avatar(
        &self,
        user_id: i64,
        avatar_url: &str,
    ) -> Result<(), sqlx::Error> {
        let sql = format!(
            "UPDATE users SET avatar_url = $1, updated_at = {} WHERE id = $2",
            self.now_expr()
        );
        sqlx::query(&sql)
            .bind(avatar_url)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn username_exists(&self, username: &str) -> Result<bool, sqlx::Error> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE username = $1")
            .bind(username)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0 > 0)
    }

    // ── Bot CRUD ──────────────────────────────────────────────────────

    pub async fn create_bot(
        &self,
        name: &str,
        description: &str,
        owner_id: Option<i64>,
    ) -> Result<Bot, sqlx::Error> {
        let row = sqlx::query_as::<_, Bot>(
            "INSERT INTO bots (name, description, owner_id) VALUES ($1, $2, $3) RETURNING id, name, description, owner_id, visibility, active_version_id, created_at, updated_at",
        )
        .bind(name)
        .bind(description)
        .bind(owner_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_bots(&self) -> Result<Vec<Bot>, sqlx::Error> {
        let rows =
            sqlx::query_as::<_, Bot>("SELECT id, name, description, owner_id, visibility, active_version_id, created_at, updated_at FROM bots ORDER BY id")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    pub async fn list_bots_by_owner(&self, owner_id: i64) -> Result<Vec<Bot>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Bot>(
            "SELECT id, name, description, owner_id, visibility, active_version_id, created_at, updated_at FROM bots WHERE owner_id = $1 ORDER BY id",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// List all bots with enriched summary data (version count, latest version, Elo).
    pub async fn list_bot_summaries(&self) -> Result<Vec<BotSummary>, sqlx::Error> {
        let rows = sqlx::query_as::<_, BotSummary>(
            r#"
            SELECT
                b.id, b.name, b.description, b.owner_id,
                u.username AS owner_username,
                b.visibility, b.created_at, b.updated_at,
                COALESCE(vs.cnt, 0) AS version_count,
                vs.max_version AS latest_version,
                vs.latest_elo AS latest_elo_1v1
            FROM bots b
            LEFT JOIN users u ON u.id = b.owner_id
            LEFT JOIN (
                SELECT bot_id,
                       COUNT(*) AS cnt,
                       MAX(version) AS max_version,
                       (SELECT bv2.elo_1v1 FROM bot_versions bv2
                        WHERE bv2.bot_id = bv.bot_id
                        ORDER BY bv2.version DESC LIMIT 1) AS latest_elo
                FROM bot_versions bv
                GROUP BY bot_id
            ) vs ON vs.bot_id = b.id
            ORDER BY b.updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// List bots by owner with enriched summary data.
    pub async fn list_bot_summaries_by_owner(
        &self,
        owner_id: i64,
    ) -> Result<Vec<BotSummary>, sqlx::Error> {
        let rows = sqlx::query_as::<_, BotSummary>(
            r#"
            SELECT
                b.id, b.name, b.description, b.owner_id,
                u.username AS owner_username,
                b.visibility, b.created_at, b.updated_at,
                COALESCE(vs.cnt, 0) AS version_count,
                vs.max_version AS latest_version,
                vs.latest_elo AS latest_elo_1v1
            FROM bots b
            LEFT JOIN users u ON u.id = b.owner_id
            LEFT JOIN (
                SELECT bot_id,
                       COUNT(*) AS cnt,
                       MAX(version) AS max_version,
                       (SELECT bv2.elo_1v1 FROM bot_versions bv2
                        WHERE bv2.bot_id = bv.bot_id
                        ORDER BY bv2.version DESC LIMIT 1) AS latest_elo
                FROM bot_versions bv
                GROUP BY bot_id
            ) vs ON vs.bot_id = b.id
            WHERE b.owner_id = $1
            ORDER BY b.updated_at DESC
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_bot(&self, id: i64) -> Result<Option<Bot>, sqlx::Error> {
        let row = sqlx::query_as::<_, Bot>(
            "SELECT id, name, description, owner_id, visibility, active_version_id, created_at, updated_at FROM bots WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_bot(
        &self,
        id: i64,
        name: &str,
        description: &str,
    ) -> Result<Option<Bot>, sqlx::Error> {
        let sql = format!(
            "UPDATE bots SET name = $1, description = $2, updated_at = {} WHERE id = $3",
            self.now_expr()
        );
        let result: AnyQueryResult = sqlx::query(&sql)
            .bind(name)
            .bind(description)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        self.get_bot(id).await
    }

    pub async fn update_bot_visibility(
        &self,
        id: i64,
        visibility: &str,
    ) -> Result<Option<Bot>, sqlx::Error> {
        let sql = format!(
            "UPDATE bots SET visibility = $1, updated_at = {} WHERE id = $2",
            self.now_expr()
        );
        let result: AnyQueryResult = sqlx::query(&sql)
            .bind(visibility)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }
        self.get_bot(id).await
    }

    pub async fn delete_bot(&self, id: i64) -> Result<bool, sqlx::Error> {
        let result: AnyQueryResult = sqlx::query("DELETE FROM bots WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ── Bot Versions ──────────────────────────────────────────────────

    pub async fn create_bot_version(
        &self,
        bot_id: i64,
        code: &str,
    ) -> Result<BotVersion, sqlx::Error> {
        // Determine next version number for this bot
        let max_version: Option<i32> =
            sqlx::query_scalar("SELECT MAX(version) FROM bot_versions WHERE bot_id = $1")
                .bind(bot_id)
                .fetch_one(&self.pool)
                .await?;

        let next_version = max_version.unwrap_or(0) + 1;

        // Soft reset: inherit Elo from parent version using (parent_elo + 1500) / 2
        let starting_elo = if next_version > 1 {
            let parent_elo: Option<i32> = sqlx::query_scalar(
                "SELECT elo_rating FROM bot_versions WHERE bot_id = $1 AND version = $2",
            )
            .bind(bot_id)
            .bind(next_version - 1)
            .fetch_one(&self.pool)
            .await?;
            let parent = parent_elo.unwrap_or(1500);
            crate::elo::soft_reset_elo(parent)
        } else {
            1500
        };

        let row = sqlx::query_as::<_, BotVersion>(
            "INSERT INTO bot_versions (bot_id, version, code, elo_rating, elo_1v1, elo_peak) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, bot_id, version, code, api_type, is_archived, is_faulty, elo_rating, elo_1v1, elo_peak, games_played, wins, losses, draws, ffa_placement_points, ffa_games, creatures_spawned, creatures_killed, creatures_lost, total_score, created_at",
        )
        .bind(bot_id)
        .bind(next_version)
        .bind(code)
        .bind(starting_elo)
        .bind(starting_elo)
        .bind(starting_elo)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_bot_versions(&self, bot_id: i64) -> Result<Vec<BotVersion>, sqlx::Error> {
        let rows = sqlx::query_as::<_, BotVersion>(
            "SELECT id, bot_id, version, code, api_type, is_archived, is_faulty, elo_rating, elo_1v1, elo_peak, games_played, wins, losses, draws, ffa_placement_points, ffa_games, creatures_spawned, creatures_killed, creatures_lost, total_score, created_at FROM bot_versions WHERE bot_id = $1 ORDER BY version",
        )
        .bind(bot_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_bot_version(
        &self,
        bot_id: i64,
        version_id: i64,
    ) -> Result<Option<BotVersion>, sqlx::Error> {
        let row = sqlx::query_as::<_, BotVersion>(
            "SELECT id, bot_id, version, code, api_type, is_archived, is_faulty, elo_rating, elo_1v1, elo_peak, games_played, wins, losses, draws, ffa_placement_points, ffa_games, creatures_spawned, creatures_killed, creatures_lost, total_score, created_at FROM bot_versions WHERE bot_id = $1 AND id = $2",
        )
        .bind(bot_id)
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Get a bot version by its ID alone (without needing bot_id).
    pub async fn get_bot_version_by_id(
        &self,
        version_id: i64,
    ) -> Result<Option<BotVersion>, sqlx::Error> {
        let row = sqlx::query_as::<_, BotVersion>(
            "SELECT id, bot_id, version, code, api_type, is_archived, is_faulty, elo_rating, elo_1v1, elo_peak, games_played, wins, losses, draws, ffa_placement_points, ffa_games, creatures_spawned, creatures_killed, creatures_lost, total_score, created_at FROM bot_versions WHERE id = $1",
        )
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    // ── Bot Version Management ──────────────────────────────────────

    pub async fn set_active_version(
        &self,
        bot_id: i64,
        version_id: i64,
    ) -> Result<bool, sqlx::Error> {
        let sql = format!(
            "UPDATE bots SET active_version_id = $1, updated_at = {} WHERE id = $2",
            self.now_expr()
        );
        let result: AnyQueryResult = sqlx::query(&sql)
            .bind(version_id)
            .bind(bot_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn archive_version(
        &self,
        version_id: i64,
        archived: bool,
    ) -> Result<bool, sqlx::Error> {
        let result: AnyQueryResult =
            sqlx::query("UPDATE bot_versions SET is_archived = $1 WHERE id = $2")
                .bind(archived as i32)
                .bind(version_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn mark_version_faulty(
        &self,
        version_id: i64,
        faulty: bool,
    ) -> Result<bool, sqlx::Error> {
        let result: AnyQueryResult =
            sqlx::query("UPDATE bot_versions SET is_faulty = $1 WHERE id = $2")
                .bind(faulty as i32)
                .bind(version_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_active_version(&self, bot_id: i64) -> Result<Option<BotVersion>, sqlx::Error> {
        let row = sqlx::query_as::<_, BotVersion>(
            "SELECT bv.id, bv.bot_id, bv.version, bv.code, bv.api_type, bv.is_archived, bv.is_faulty, bv.elo_rating, bv.elo_1v1, bv.elo_peak, bv.games_played, bv.wins, bv.losses, bv.draws, bv.ffa_placement_points, bv.ffa_games, bv.creatures_spawned, bv.creatures_killed, bv.creatures_lost, bv.total_score, bv.created_at FROM bot_versions bv JOIN bots b ON b.active_version_id = bv.id WHERE b.id = $1",
        )
        .bind(bot_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    // ── Match Recording ──────────────────────────────────────────────

    pub async fn create_match(&self, format: &str, map: &str) -> Result<Match, sqlx::Error> {
        let row = sqlx::query_as::<_, Match>(
            "INSERT INTO matches (format, map, status) VALUES ($1, $2, 'running') RETURNING id, format, map, status, winner_bot_version_id, created_at, finished_at",
        )
        .bind(format)
        .bind(map)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Mark any matches still in 'running' status as 'abandoned'.
    /// Called at startup to clean up orphaned matches from prior server runs.
    pub async fn cleanup_orphaned_matches(&self) -> Result<u64, sqlx::Error> {
        let sql = format!(
            "UPDATE matches SET status = 'abandoned', finished_at = {} WHERE status = 'running'",
            self.now_expr()
        );
        let result: AnyQueryResult = sqlx::query(&sql).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    /// Mark any tournaments still in 'running' status as 'abandoned'.
    pub async fn cleanup_orphaned_tournaments(&self) -> Result<u64, sqlx::Error> {
        let result: AnyQueryResult =
            sqlx::query("UPDATE tournaments SET status = 'abandoned' WHERE status = 'running'")
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected())
    }

    pub async fn finish_match(
        &self,
        match_id: i64,
        winner_bot_version_id: Option<i64>,
    ) -> Result<bool, sqlx::Error> {
        let sql = format!(
            "UPDATE matches SET status = 'finished', winner_bot_version_id = $1, finished_at = {} WHERE id = $2",
            self.now_expr()
        );
        let result: AnyQueryResult = sqlx::query(&sql)
            .bind(winner_bot_version_id)
            .bind(match_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_match(&self, id: i64) -> Result<Option<Match>, sqlx::Error> {
        let row = sqlx::query_as::<_, Match>(
            "SELECT id, format, map, status, winner_bot_version_id, created_at, finished_at FROM matches WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_recent_matches(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Match>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Match>(
            "SELECT id, format, map, status, winner_bot_version_id, created_at, finished_at FROM matches ORDER BY id DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_matches_filtered(
        &self,
        limit: i64,
        offset: i64,
        bot_id: Option<i64>,
        user_id: Option<i64>,
        username: Option<&str>,
        status: Option<&str>,
        map: Option<&str>,
        sort: &str,
    ) -> Result<Vec<Match>, sqlx::Error> {
        let needs_join = bot_id.is_some() || user_id.is_some() || username.is_some();

        let mut sql = String::from(
            "SELECT DISTINCT m.id, m.format, m.map, m.status, m.winner_bot_version_id, m.created_at, m.finished_at FROM matches m",
        );

        if needs_join {
            sql.push_str(
                " JOIN match_participants mp ON mp.match_id = m.id \
                 JOIN bot_versions bv ON bv.id = mp.bot_version_id \
                 JOIN bots b ON b.id = bv.bot_id",
            );
            if username.is_some() {
                sql.push_str(" JOIN users u ON u.id = b.owner_id");
            }
        }

        let mut conditions: Vec<String> = Vec::new();
        let mut param_index: usize = 0;

        if bot_id.is_some() {
            param_index += 1;
            conditions.push(format!("b.id = ${param_index}"));
        }
        if user_id.is_some() {
            param_index += 1;
            conditions.push(format!("b.owner_id = ${param_index}"));
        }
        if username.is_some() {
            param_index += 1;
            conditions.push(format!("u.username = ${param_index}"));
        }
        if status.is_some() {
            param_index += 1;
            conditions.push(format!("m.status = ${param_index}"));
        }
        if map.is_some() {
            param_index += 1;
            conditions.push(format!("m.map = ${param_index}"));
        }

        if !conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conditions.join(" AND "));
        }

        let order = if sort == "oldest" { "ASC" } else { "DESC" };
        param_index += 1;
        let limit_idx = param_index;
        param_index += 1;
        let offset_idx = param_index;
        sql.push_str(&format!(
            " ORDER BY m.id {order} LIMIT ${limit_idx} OFFSET ${offset_idx}"
        ));

        let mut query = sqlx::query_as::<_, Match>(&sql);

        if let Some(bid) = bot_id {
            query = query.bind(bid);
        }
        if let Some(uid) = user_id {
            query = query.bind(uid);
        }
        if let Some(uname) = username {
            query = query.bind(uname.to_string());
        }
        if let Some(st) = status {
            query = query.bind(st.to_string());
        }
        if let Some(m) = map {
            query = query.bind(m.to_string());
        }
        query = query.bind(limit);
        query = query.bind(offset);

        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows)
    }

    /// Fetch player names (bot names) for a set of match IDs.
    /// Returns a Vec of (match_id, bot_name) pairs.
    pub async fn get_match_player_names(
        &self,
        match_ids: &[i64],
    ) -> Result<Vec<(i64, String)>, sqlx::Error> {
        if match_ids.is_empty() {
            return Ok(vec![]);
        }
        // Build placeholder list for ANY-style query
        // sqlx doesn't support binding slices for all backends, so we build the IN list manually
        let placeholders: Vec<String> = match_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("${}", i + 1))
            .collect();
        let sql = format!(
            "SELECT mp.match_id, b.name \
             FROM match_participants mp \
             JOIN bot_versions bv ON bv.id = mp.bot_version_id \
             JOIN bots b ON b.id = bv.bot_id \
             WHERE mp.match_id IN ({}) \
             ORDER BY mp.match_id, mp.player_slot",
            placeholders.join(", ")
        );
        let mut query = sqlx::query_as::<_, (i64, String)>(&sql);
        for id in match_ids {
            query = query.bind(*id);
        }
        let rows = query.fetch_all(&self.pool).await?;
        Ok(rows)
    }

    pub async fn list_user_matches(
        &self,
        user_id: i64,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Match>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Match>(
            "SELECT DISTINCT m.id, m.format, m.map, m.status, m.winner_bot_version_id, m.created_at, m.finished_at \
             FROM matches m \
             JOIN match_participants mp ON mp.match_id = m.id \
             JOIN bot_versions bv ON bv.id = mp.bot_version_id \
             JOIN bots b ON b.id = bv.bot_id \
             WHERE b.owner_id = $1 \
             ORDER BY m.id DESC \
             LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn add_match_participant(
        &self,
        match_id: i64,
        bot_version_id: i64,
        player_slot: i32,
    ) -> Result<MatchParticipant, sqlx::Error> {
        let row = sqlx::query_as::<_, MatchParticipant>(
            "INSERT INTO match_participants (match_id, bot_version_id, player_slot) VALUES ($1, $2, $3) RETURNING id, match_id, bot_version_id, player_slot, final_score, placement, elo_before, elo_after, creatures_spawned, creatures_killed, creatures_lost, NULL AS bot_name, NULL AS owner_name",
        )
        .bind(match_id)
        .bind(bot_version_id)
        .bind(player_slot)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_match_participant(
        &self,
        participant_id: i64,
        final_score: i32,
        placement: Option<i32>,
        elo_before: Option<i32>,
        elo_after: Option<i32>,
        creatures_spawned: i32,
        creatures_killed: i32,
        creatures_lost: i32,
    ) -> Result<bool, sqlx::Error> {
        let result: AnyQueryResult = sqlx::query(
            "UPDATE match_participants SET final_score = $1, placement = $2, elo_before = $3, elo_after = $4, creatures_spawned = $5, creatures_killed = $6, creatures_lost = $7 WHERE id = $8",
        )
        .bind(final_score)
        .bind(placement)
        .bind(elo_before)
        .bind(elo_after)
        .bind(creatures_spawned)
        .bind(creatures_killed)
        .bind(creatures_lost)
        .bind(participant_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_match_participants(
        &self,
        match_id: i64,
    ) -> Result<Vec<MatchParticipant>, sqlx::Error> {
        let rows = sqlx::query_as::<_, MatchParticipant>(
            "SELECT mp.id, mp.match_id, mp.bot_version_id, mp.player_slot, mp.final_score, mp.placement, mp.elo_before, mp.elo_after, mp.creatures_spawned, mp.creatures_killed, mp.creatures_lost, b.name AS bot_name, u.username AS owner_name FROM match_participants mp LEFT JOIN bot_versions bv ON bv.id = mp.bot_version_id LEFT JOIN bots b ON b.id = bv.bot_id LEFT JOIN users u ON u.id = b.owner_id WHERE mp.match_id = $1 ORDER BY mp.player_slot",
        )
        .bind(match_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ── Elo & Stats Updates ──────────────────────────────────────────

    pub async fn update_version_elo(
        &self,
        version_id: i64,
        new_elo: i32,
    ) -> Result<bool, sqlx::Error> {
        // GREATEST works on PostgreSQL; for SQLite MAX works in this context too
        let max_fn = if self.is_postgres { "GREATEST" } else { "MAX" };
        let sql = format!(
            "UPDATE bot_versions SET elo_rating = $1, elo_1v1 = $2, elo_peak = {max_fn}(elo_peak, $3) WHERE id = $4"
        );
        let result: AnyQueryResult = sqlx::query(&sql)
            .bind(new_elo)
            .bind(new_elo)
            .bind(new_elo)
            .bind(version_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_version_stats(
        &self,
        version_id: i64,
        won: bool,
        lost: bool,
        draw: bool,
        score: i32,
        spawned: i32,
        killed: i32,
        died: i32,
    ) -> Result<bool, sqlx::Error> {
        let result: AnyQueryResult = sqlx::query(
            "UPDATE bot_versions SET games_played = games_played + 1, wins = wins + $1, losses = losses + $2, draws = draws + $3, total_score = total_score + $4, creatures_spawned = creatures_spawned + $5, creatures_killed = creatures_killed + $6, creatures_lost = creatures_lost + $7 WHERE id = $8",
        )
        .bind(won as i32)
        .bind(lost as i32)
        .bind(draw as i32)
        .bind(score as i64)
        .bind(spawned)
        .bind(killed)
        .bind(died)
        .bind(version_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn update_version_ffa_stats(
        &self,
        version_id: i64,
        placement_points: i32,
    ) -> Result<bool, sqlx::Error> {
        let result: AnyQueryResult = sqlx::query(
            "UPDATE bot_versions SET ffa_games = ffa_games + 1, ffa_placement_points = ffa_placement_points + $1 WHERE id = $2",
        )
        .bind(placement_points)
        .bind(version_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_bot_version_stats(
        &self,
        version_id: i64,
    ) -> Result<Option<BotVersion>, sqlx::Error> {
        self.get_bot_version_by_id(version_id).await
    }

    // ── Tournament CRUD ───────────────────────────────────────────────

    pub async fn create_tournament(
        &self,
        name: &str,
        map: &str,
    ) -> Result<Tournament, sqlx::Error> {
        let row = sqlx::query_as::<_, Tournament>(
            "INSERT INTO tournaments (name, map) VALUES ($1, $2) RETURNING id, name, status, map, config, format, current_round, total_rounds, created_at",
        )
        .bind(name)
        .bind(map)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_tournaments(&self) -> Result<Vec<Tournament>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Tournament>(
            "SELECT id, name, status, map, config, format, current_round, total_rounds, created_at FROM tournaments ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_tournament(&self, id: i64) -> Result<Option<Tournament>, sqlx::Error> {
        let row = sqlx::query_as::<_, Tournament>(
            "SELECT id, name, status, map, config, format, current_round, total_rounds, created_at FROM tournaments WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_tournament_status(
        &self,
        id: i64,
        status: &str,
    ) -> Result<bool, sqlx::Error> {
        let result: AnyQueryResult =
            sqlx::query("UPDATE tournaments SET status = $1 WHERE id = $2")
                .bind(status)
                .bind(id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn update_tournament_round(&self, id: i64, round: i32) -> Result<bool, sqlx::Error> {
        let result: AnyQueryResult =
            sqlx::query("UPDATE tournaments SET current_round = $1 WHERE id = $2")
                .bind(round)
                .bind(id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn update_tournament(
        &self,
        id: i64,
        name: Option<&str>,
        map: Option<&str>,
        format: Option<&str>,
        config: Option<&str>,
    ) -> Result<Option<Tournament>, sqlx::Error> {
        let result: AnyQueryResult = sqlx::query(
            "UPDATE tournaments SET name = COALESCE($1, name), map = COALESCE($2, map), format = COALESCE($3, format), config = COALESCE($4, config) WHERE id = $5",
        )
        .bind(name)
        .bind(map)
        .bind(format)
        .bind(config)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }
        self.get_tournament(id).await
    }

    pub async fn get_tournament_standings(
        &self,
        tournament_id: i64,
    ) -> Result<Vec<TournamentStanding>, sqlx::Error> {
        // Aggregate results by bot_version_id, joining with bots table for name
        let rows: Vec<(i64, String, i64, i32)> = sqlx::query_as(
            r#"
            SELECT
                tr.bot_version_id,
                COALESCE(b.name, 'Unknown') AS bot_name,
                COALESCE(SUM(tr.final_score), 0) AS total_score,
                COUNT(*) AS matches_played
            FROM tournament_results tr
            JOIN bot_versions bv ON bv.id = tr.bot_version_id
            JOIN bots b ON b.id = bv.bot_id
            WHERE tr.tournament_id = $1
            GROUP BY tr.bot_version_id
            ORDER BY total_score DESC
            "#,
        )
        .bind(tournament_id)
        .fetch_all(&self.pool)
        .await?;

        // Compute wins/losses from the results
        // A "win" means the bot had the highest score in a given result batch
        // For simplicity, we compute wins as the count of times this bot had the
        // max score among results with the same finished_at timestamp
        let all_results = self.get_tournament_results(tournament_id).await?;

        // Group results by finished_at (each game produces results with same timestamp)
        let mut games: std::collections::HashMap<String, Vec<&TournamentResult>> =
            std::collections::HashMap::new();
        for r in &all_results {
            games.entry(r.finished_at.clone()).or_default().push(r);
        }

        // Count wins/losses per bot_version_id
        let mut wins_map: std::collections::HashMap<i64, i32> = std::collections::HashMap::new();
        let mut losses_map: std::collections::HashMap<i64, i32> = std::collections::HashMap::new();
        for game_results in games.values() {
            if game_results.is_empty() {
                continue;
            }
            let max_score = game_results
                .iter()
                .map(|r| r.final_score)
                .max()
                .unwrap_or(0);
            for r in game_results {
                if r.final_score == max_score {
                    *wins_map.entry(r.bot_version_id).or_default() += 1;
                } else {
                    *losses_map.entry(r.bot_version_id).or_default() += 1;
                }
            }
        }

        let standings = rows
            .into_iter()
            .map(
                |(bot_version_id, bot_name, total_score, matches_played)| TournamentStanding {
                    bot_version_id,
                    bot_name,
                    total_score,
                    matches_played,
                    wins: *wins_map.get(&bot_version_id).unwrap_or(&0),
                    losses: *losses_map.get(&bot_version_id).unwrap_or(&0),
                },
            )
            .collect();

        Ok(standings)
    }

    // ── Tournament Entries ────────────────────────────────────────────

    pub async fn add_tournament_entry(
        &self,
        tournament_id: i64,
        bot_version_id: i64,
        slot_name: &str,
    ) -> Result<TournamentEntry, sqlx::Error> {
        let row = sqlx::query_as::<_, TournamentEntry>(
            "INSERT INTO tournament_entries (tournament_id, bot_version_id, slot_name) VALUES ($1, $2, $3) RETURNING id, tournament_id, bot_version_id, slot_name, NULL AS bot_name, NULL AS version",
        )
        .bind(tournament_id)
        .bind(bot_version_id)
        .bind(slot_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_tournament_entries(
        &self,
        tournament_id: i64,
    ) -> Result<Vec<TournamentEntry>, sqlx::Error> {
        let rows = sqlx::query_as::<_, TournamentEntry>(
            "SELECT te.id, te.tournament_id, te.bot_version_id, te.slot_name, b.name AS bot_name, bv.version FROM tournament_entries te LEFT JOIN bot_versions bv ON bv.id = te.bot_version_id LEFT JOIN bots b ON b.id = bv.bot_id WHERE te.tournament_id = $1 ORDER BY te.id",
        )
        .bind(tournament_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn remove_tournament_entry(&self, entry_id: i64) -> Result<bool, sqlx::Error> {
        let result: AnyQueryResult = sqlx::query("DELETE FROM tournament_entries WHERE id = $1")
            .bind(entry_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ── Tournament Results ────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn add_tournament_result(
        &self,
        tournament_id: i64,
        player_slot: i32,
        bot_version_id: i64,
        final_score: i32,
        creatures_spawned: i32,
        creatures_killed: i32,
        creatures_lost: i32,
    ) -> Result<TournamentResult, sqlx::Error> {
        let row = sqlx::query_as::<_, TournamentResult>(
            "INSERT INTO tournament_results (tournament_id, player_slot, bot_version_id, final_score, creatures_spawned, creatures_killed, creatures_lost) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id, tournament_id, player_slot, bot_version_id, final_score, creatures_spawned, creatures_killed, creatures_lost, finished_at",
        )
        .bind(tournament_id)
        .bind(player_slot)
        .bind(bot_version_id)
        .bind(final_score)
        .bind(creatures_spawned)
        .bind(creatures_killed)
        .bind(creatures_lost)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn get_tournament_results(
        &self,
        tournament_id: i64,
    ) -> Result<Vec<TournamentResult>, sqlx::Error> {
        let rows = sqlx::query_as::<_, TournamentResult>(
            "SELECT id, tournament_id, player_slot, bot_version_id, final_score, creatures_spawned, creatures_killed, creatures_lost, finished_at FROM tournament_results WHERE tournament_id = $1 ORDER BY final_score DESC",
        )
        .bind(tournament_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ── Tournament-Match Linking ────────────────────────────────────

    pub async fn add_tournament_match(
        &self,
        tournament_id: i64,
        match_id: i64,
        round: i32,
    ) -> Result<TournamentMatch, sqlx::Error> {
        let row = sqlx::query_as::<_, TournamentMatch>(
            "INSERT INTO tournament_matches (tournament_id, match_id, round) VALUES ($1, $2, $3) RETURNING id, tournament_id, match_id, round",
        )
        .bind(tournament_id)
        .bind(match_id)
        .bind(round)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_tournament_matches(
        &self,
        tournament_id: i64,
    ) -> Result<Vec<TournamentMatch>, sqlx::Error> {
        let rows = sqlx::query_as::<_, TournamentMatch>(
            "SELECT id, tournament_id, match_id, round FROM tournament_matches WHERE tournament_id = $1 ORDER BY round, id",
        )
        .bind(tournament_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_tournament_matches_by_round(
        &self,
        tournament_id: i64,
        round: i32,
    ) -> Result<Vec<TournamentMatch>, sqlx::Error> {
        let rows = sqlx::query_as::<_, TournamentMatch>(
            "SELECT id, tournament_id, match_id, round FROM tournament_matches WHERE tournament_id = $1 AND round = $2 ORDER BY id",
        )
        .bind(tournament_id)
        .bind(round)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_tournament_matches_detail(
        &self,
        tournament_id: i64,
    ) -> Result<Vec<TournamentMatchDetail>, sqlx::Error> {
        let tm_rows = self.list_tournament_matches(tournament_id).await?;
        let mut details = Vec::new();
        for tm in &tm_rows {
            let m = self.get_match(tm.match_id).await?;
            let match_info = match m {
                Some(m) => m,
                None => continue,
            };
            let participants = sqlx::query_as::<_, TournamentMatchParticipantInfo>(
                "SELECT mp.match_id, mp.bot_version_id, mp.player_slot, mp.final_score, b.name AS bot_name, u.username AS owner_name FROM match_participants mp LEFT JOIN bot_versions bv ON bv.id = mp.bot_version_id LEFT JOIN bots b ON b.id = bv.bot_id LEFT JOIN users u ON u.id = b.owner_id WHERE mp.match_id = $1 ORDER BY mp.player_slot",
            )
            .bind(tm.match_id)
            .fetch_all(&self.pool)
            .await?;

            details.push(TournamentMatchDetail {
                match_id: tm.match_id,
                round: tm.round,
                status: match_info.status,
                winner_bot_version_id: match_info.winner_bot_version_id,
                finished_at: match_info.finished_at,
                participants,
            });
        }
        Ok(details)
    }

    /// Returns (tournament_id, round) for a match if it belongs to a tournament.
    pub async fn get_tournament_for_match(
        &self,
        match_id: i64,
    ) -> Result<Option<(i64, i32)>, sqlx::Error> {
        let row = sqlx::query_as::<_, (i64, i32)>(
            "SELECT tournament_id, round FROM tournament_matches WHERE match_id = $1 LIMIT 1",
        )
        .bind(match_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    // ── Leaderboards ─────────────────────────────────────────────────

    pub async fn leaderboard_1v1(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        // CAST AS REAL works on SQLite; PostgreSQL needs DOUBLE PRECISION
        let cast_type = if self.is_postgres {
            "DOUBLE PRECISION"
        } else {
            "REAL"
        };
        let sql = format!(
            r#"
            SELECT
                ROW_NUMBER() OVER (ORDER BY bv.elo_1v1 DESC) AS rank,
                bv.id AS bot_version_id,
                b.name AS bot_name,
                bv.version,
                COALESCE(u.username, 'anonymous') AS owner_username,
                bv.elo_1v1 AS rating,
                bv.games_played,
                bv.wins,
                bv.losses,
                CASE WHEN bv.games_played > 0
                    THEN CAST(bv.wins AS {cast_type}) / bv.games_played
                    ELSE 0.0
                END AS win_rate
            FROM bot_versions bv
            JOIN bots b ON b.id = bv.bot_id
            LEFT JOIN users u ON u.id = b.owner_id
            WHERE bv.is_archived = 0 AND bv.games_played > 0
            ORDER BY bv.elo_1v1 DESC
            LIMIT $1 OFFSET $2
            "#
        );
        let rows = sqlx::query_as::<_, LeaderboardEntry>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    pub async fn leaderboard_ffa(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        let cast_type = if self.is_postgres {
            "DOUBLE PRECISION"
        } else {
            "REAL"
        };
        let sql = format!(
            r#"
            SELECT
                ROW_NUMBER() OVER (ORDER BY bv.ffa_placement_points DESC) AS rank,
                bv.id AS bot_version_id,
                b.name AS bot_name,
                bv.version,
                COALESCE(u.username, 'anonymous') AS owner_username,
                bv.ffa_placement_points AS rating,
                bv.games_played,
                bv.wins,
                bv.losses,
                CASE WHEN bv.games_played > 0
                    THEN CAST(bv.wins AS {cast_type}) / bv.games_played
                    ELSE 0.0
                END AS win_rate
            FROM bot_versions bv
            JOIN bots b ON b.id = bv.bot_id
            LEFT JOIN users u ON u.id = b.owner_id
            WHERE bv.is_archived = 0 AND bv.games_played > 0
            ORDER BY bv.ffa_placement_points DESC
            LIMIT $1 OFFSET $2
            "#
        );
        let rows = sqlx::query_as::<_, LeaderboardEntry>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    // ── Team CRUD ─────────────────────────────────────────────────────

    pub async fn create_team(&self, owner_id: i64, name: &str) -> Result<Team, sqlx::Error> {
        let row = sqlx::query_as::<_, Team>(
            "INSERT INTO teams (owner_id, name) VALUES ($1, $2) RETURNING id, owner_id, name, created_at",
        )
        .bind(owner_id)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_teams_by_owner(&self, owner_id: i64) -> Result<Vec<Team>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Team>(
            "SELECT id, owner_id, name, created_at FROM teams WHERE owner_id = $1 ORDER BY id",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_team(&self, id: i64) -> Result<Option<Team>, sqlx::Error> {
        let row = sqlx::query_as::<_, Team>(
            "SELECT id, owner_id, name, created_at FROM teams WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_team_name(&self, id: i64, name: &str) -> Result<Option<Team>, sqlx::Error> {
        let result: AnyQueryResult = sqlx::query("UPDATE teams SET name = $1 WHERE id = $2")
            .bind(name)
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        self.get_team(id).await
    }

    pub async fn delete_team(&self, id: i64) -> Result<bool, sqlx::Error> {
        let result: AnyQueryResult = sqlx::query("DELETE FROM teams WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    // ── Team Versions ────────────────────────────────────────────────

    pub async fn create_team_version(
        &self,
        team_id: i64,
        bot_version_a: i64,
        bot_version_b: i64,
    ) -> Result<TeamVersion, sqlx::Error> {
        let max_version: Option<i32> =
            sqlx::query_scalar("SELECT MAX(version) FROM team_versions WHERE team_id = $1")
                .bind(team_id)
                .fetch_one(&self.pool)
                .await?;

        let next_version = max_version.unwrap_or(0) + 1;

        let row = sqlx::query_as::<_, TeamVersion>(
            "INSERT INTO team_versions (team_id, version, bot_version_a, bot_version_b) VALUES ($1, $2, $3, $4) RETURNING id, team_id, version, bot_version_a, bot_version_b, elo_rating, games_played, wins, losses, draws, created_at",
        )
        .bind(team_id)
        .bind(next_version)
        .bind(bot_version_a)
        .bind(bot_version_b)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_team_versions(&self, team_id: i64) -> Result<Vec<TeamVersion>, sqlx::Error> {
        let rows = sqlx::query_as::<_, TeamVersion>(
            "SELECT id, team_id, version, bot_version_a, bot_version_b, elo_rating, games_played, wins, losses, draws, created_at FROM team_versions WHERE team_id = $1 ORDER BY version",
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_team_version(
        &self,
        team_id: i64,
        version_id: i64,
    ) -> Result<Option<TeamVersion>, sqlx::Error> {
        let row = sqlx::query_as::<_, TeamVersion>(
            "SELECT id, team_id, version, bot_version_a, bot_version_b, elo_rating, games_played, wins, losses, draws, created_at FROM team_versions WHERE team_id = $1 AND id = $2",
        )
        .bind(team_id)
        .bind(version_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn leaderboard_2v2(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
        let cast_type = if self.is_postgres {
            "DOUBLE PRECISION"
        } else {
            "REAL"
        };
        let sql = format!(
            r#"
            SELECT
                ROW_NUMBER() OVER (ORDER BY tv.elo_rating DESC) AS rank,
                tv.id AS bot_version_id,
                t.name AS bot_name,
                tv.version,
                COALESCE(u.username, 'anonymous') AS owner_username,
                tv.elo_rating AS rating,
                tv.games_played,
                tv.wins,
                tv.losses,
                CASE WHEN tv.games_played > 0
                    THEN CAST(tv.wins AS {cast_type}) / tv.games_played
                    ELSE 0.0
                END AS win_rate
            FROM team_versions tv
            JOIN teams t ON t.id = tv.team_id
            LEFT JOIN users u ON u.id = t.owner_id
            WHERE tv.games_played > 0
            ORDER BY tv.elo_rating DESC
            LIMIT $1 OFFSET $2
            "#
        );
        let rows = sqlx::query_as::<_, LeaderboardEntry>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    // ── API Token CRUD ──────────────────────────────────────────────────

    pub async fn create_api_token(
        &self,
        user_id: i64,
        name: &str,
        token_hash: &str,
        scopes: &str,
    ) -> Result<ApiToken, sqlx::Error> {
        let row = sqlx::query_as::<_, ApiToken>(
            "INSERT INTO api_tokens (user_id, name, token_hash, scopes) VALUES ($1, $2, $3, $4) RETURNING id, user_id, name, token_hash, scopes, last_used_at, created_at",
        )
        .bind(user_id)
        .bind(name)
        .bind(token_hash)
        .bind(scopes)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_api_tokens(&self, user_id: i64) -> Result<Vec<ApiToken>, sqlx::Error> {
        let rows = sqlx::query_as::<_, ApiToken>(
            "SELECT id, user_id, name, token_hash, scopes, last_used_at, created_at FROM api_tokens WHERE user_id = $1 ORDER BY id",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn delete_api_token(&self, id: i64, user_id: i64) -> Result<bool, sqlx::Error> {
        let result: AnyQueryResult =
            sqlx::query("DELETE FROM api_tokens WHERE id = $1 AND user_id = $2")
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_api_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<ApiToken>, sqlx::Error> {
        let row = sqlx::query_as::<_, ApiToken>(
            "SELECT id, user_id, name, token_hash, scopes, last_used_at, created_at FROM api_tokens WHERE token_hash = $1",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_token_last_used(&self, id: i64) -> Result<bool, sqlx::Error> {
        let sql = format!(
            "UPDATE api_tokens SET last_used_at = {} WHERE id = $1",
            self.now_expr()
        );
        let result: AnyQueryResult = sqlx::query(&sql).bind(id).execute(&self.pool).await?;
        Ok(result.rows_affected() > 0)
    }

    // ── Replay CRUD ──────────────────────────────────────────────────

    pub async fn save_replay(
        &self,
        match_id: i64,
        data: &[u8],
        tick_count: i32,
    ) -> Result<Replay, sqlx::Error> {
        let row = sqlx::query_as::<_, Replay>(
            "INSERT INTO replays (match_id, data, tick_count) VALUES ($1, $2, $3) RETURNING id, match_id, data, tick_count, created_at",
        )
        .bind(match_id)
        .bind(data)
        .bind(tick_count)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn get_replay(&self, match_id: i64) -> Result<Option<Replay>, sqlx::Error> {
        let row = sqlx::query_as::<_, Replay>(
            "SELECT id, match_id, data, tick_count, created_at FROM replays WHERE match_id = $1",
        )
        .bind(match_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    // ── Notification CRUD ────────────────────────────────────────────

    pub async fn create_notification(
        &self,
        user_id: i64,
        notification_type: &str,
        title: &str,
        message: &str,
        data: Option<&str>,
    ) -> Result<Notification, sqlx::Error> {
        let row = sqlx::query_as::<_, Notification>(
            "INSERT INTO notifications (user_id, type, title, message, data) VALUES ($1, $2, $3, $4, $5) RETURNING id, user_id, type, title, message, data, read, created_at",
        )
        .bind(user_id)
        .bind(notification_type)
        .bind(title)
        .bind(message)
        .bind(data)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_unread_notifications(
        &self,
        user_id: i64,
    ) -> Result<Vec<Notification>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Notification>(
            "SELECT id, user_id, type, title, message, data, read, created_at FROM notifications WHERE user_id = $1 AND read = 0 ORDER BY id DESC LIMIT 50",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_recent_notifications(
        &self,
        user_id: i64,
        limit: i64,
    ) -> Result<Vec<Notification>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Notification>(
            "SELECT id, user_id, type, title, message, data, read, created_at FROM notifications WHERE user_id = $1 ORDER BY id DESC LIMIT $2",
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn mark_notification_read(&self, id: i64, user_id: i64) -> Result<bool, sqlx::Error> {
        let result =
            sqlx::query("UPDATE notifications SET read = 1 WHERE id = $1 AND user_id = $2")
                .bind(id)
                .bind(user_id)
                .execute(&self.pool)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn unread_notification_count(&self, user_id: i64) -> Result<i64, sqlx::Error> {
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND read = 0")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0)
    }

    /// Get all unique owner IDs for bot versions involved in a match.
    pub async fn get_match_participant_owner_ids(
        &self,
        match_id: i64,
    ) -> Result<Vec<i64>, sqlx::Error> {
        let rows: Vec<(i64,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT b.owner_id
            FROM match_participants mp
            JOIN bot_versions bv ON bv.id = mp.bot_version_id
            JOIN bots b ON b.id = bv.bot_id
            WHERE mp.match_id = $1 AND b.owner_id IS NOT NULL
            "#,
        )
        .bind(match_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    // ── Feedback CRUD ─────────────────────────────────────────────────

    pub async fn create_feedback(
        &self,
        user_id: Option<i64>,
        category: &str,
        description: &str,
    ) -> Result<Feedback, sqlx::Error> {
        let row = sqlx::query_as::<_, Feedback>(
            "INSERT INTO feedback (user_id, category, description) VALUES ($1, $2, $3) RETURNING id, user_id, category, description, created_at",
        )
        .bind(user_id)
        .bind(category)
        .bind(description)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_feedback(&self) -> Result<Vec<Feedback>, sqlx::Error> {
        let rows = sqlx::query_as::<_, Feedback>(
            "SELECT id, user_id, category, description, created_at FROM feedback ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // ── Game Queue ───────────────────────────────────────────────────

    /// Enqueue a game for processing by the worker pool.
    pub async fn enqueue_game(
        &self,
        match_id: i64,
        map: Option<&str>,
        priority: i32,
        map_params: Option<&str>,
    ) -> Result<QueueJob, sqlx::Error> {
        let row = sqlx::query_as::<_, QueueJob>(
            r#"INSERT INTO game_queue (match_id, map, priority, map_params)
               VALUES ($1, $2, $3, $4)
               RETURNING id, match_id, status, worker_id, map, map_params, priority,
                         attempts, max_attempts, error_message,
                         claimed_at, completed_at, created_at"#,
        )
        .bind(match_id)
        .bind(map)
        .bind(priority)
        .bind(map_params)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Atomically claim the next pending job for a worker.
    /// Uses advisory locking on PostgreSQL; on SQLite the single-writer
    /// serialization provides equivalent safety.
    pub async fn claim_queue_job(&self, worker_id: &str) -> Result<Option<QueueJob>, sqlx::Error> {
        let now = self.now_expr();
        if self.is_postgres {
            // Use FOR UPDATE SKIP LOCKED for safe concurrent claiming
            let sql = format!(
                r#"UPDATE game_queue SET status = 'claimed', worker_id = $1, claimed_at = {now}
                   WHERE id = (
                       SELECT id FROM game_queue
                       WHERE status = 'pending'
                       ORDER BY priority DESC, created_at ASC
                       LIMIT 1
                       FOR UPDATE SKIP LOCKED
                   )
                   RETURNING id, match_id, status, worker_id, map, map_params, priority,
                             attempts, max_attempts, error_message,
                             claimed_at, completed_at, created_at"#,
            );
            let row = sqlx::query_as::<_, QueueJob>(&sql)
                .bind(worker_id)
                .fetch_optional(&self.pool)
                .await?;
            Ok(row)
        } else {
            // SQLite: single writer, no SKIP LOCKED needed
            let sql = format!(
                r#"UPDATE game_queue SET status = 'claimed', worker_id = ?, claimed_at = {now}
                   WHERE id = (
                       SELECT id FROM game_queue
                       WHERE status = 'pending'
                       ORDER BY priority DESC, created_at ASC
                       LIMIT 1
                   )
                   RETURNING id, match_id, status, worker_id, map, map_params, priority,
                             attempts, max_attempts, error_message,
                             claimed_at, completed_at, created_at"#,
            );
            let row = sqlx::query_as::<_, QueueJob>(&sql)
                .bind(worker_id)
                .fetch_optional(&self.pool)
                .await?;
            Ok(row)
        }
    }

    /// Mark a queue job as completed.
    pub async fn complete_queue_job(&self, job_id: i64) -> Result<(), sqlx::Error> {
        let now = self.now_expr();
        let sql = format!(
            "UPDATE game_queue SET status = 'completed', completed_at = {now} WHERE id = $1"
        );
        sqlx::query(&sql).bind(job_id).execute(&self.pool).await?;
        Ok(())
    }

    /// Mark a queue job as failed. If attempts < max_attempts, re-queue it as pending.
    pub async fn fail_queue_job(&self, job_id: i64, error: &str) -> Result<(), sqlx::Error> {
        // Increment attempts and set error message
        sqlx::query(
            r#"UPDATE game_queue
               SET attempts = attempts + 1, error_message = $1
               WHERE id = $2"#,
        )
        .bind(error)
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        // If under max attempts, re-queue; otherwise mark failed
        sqlx::query(
            r#"UPDATE game_queue
               SET status = CASE
                   WHEN attempts < max_attempts THEN 'pending'
                   ELSE 'failed'
               END,
               worker_id = NULL,
               claimed_at = NULL
               WHERE id = $1"#,
        )
        .bind(job_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get queue depth and status counts.
    pub async fn queue_status(&self) -> Result<GameQueueStatus, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct StatusCount {
            status: String,
            cnt: i64,
        }
        let rows: Vec<StatusCount> =
            sqlx::query_as("SELECT status, COUNT(*) as cnt FROM game_queue GROUP BY status")
                .fetch_all(&self.pool)
                .await?;

        let mut pending = 0i64;
        let mut claimed = 0i64;
        let mut completed = 0i64;
        let mut failed = 0i64;
        for r in &rows {
            match r.status.as_str() {
                "pending" => pending = r.cnt,
                "claimed" => claimed = r.cnt,
                "completed" => completed = r.cnt,
                "failed" => failed = r.cnt,
                _ => {}
            }
        }
        Ok(GameQueueStatus {
            pending,
            claimed,
            completed,
            failed,
            total: pending + claimed + completed + failed,
        })
    }

    /// Reset any jobs that were claimed but never completed (e.g. from a crashed worker).
    /// Resets jobs claimed more than 30 minutes ago back to pending.
    pub async fn cleanup_stale_queue_jobs(&self) -> Result<u64, sqlx::Error> {
        let sql = if self.is_postgres {
            r#"UPDATE game_queue SET status = 'pending', worker_id = NULL, claimed_at = NULL
               WHERE status = 'claimed'
               AND claimed_at < (now() - interval '30 minutes')::text"#
        } else {
            r#"UPDATE game_queue SET status = 'pending', worker_id = NULL, claimed_at = NULL
               WHERE status = 'claimed'
               AND claimed_at < datetime('now', '-30 minutes')"#
        };
        let result = sqlx::query(sql).execute(&self.pool).await?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_db() -> Database {
        // Install Any driver support (safe to call multiple times)
        sqlx::any::install_default_drivers();
        Database::new("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn test_create_and_get_user() {
        let db = test_db().await;

        let user = db
            .create_user("testuser", "test@example.com", "hashedpw", "Test User")
            .await
            .unwrap();
        assert_eq!(user.username, "testuser");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.role, "user");

        let fetched = db.get_user(user.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().username, "testuser");

        let by_name = db.get_user_by_username("testuser").await.unwrap();
        assert!(by_name.is_some());

        let missing = db.get_user_by_username("nonexistent").await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_user_unique_constraints() {
        let db = test_db().await;

        db.create_user("user1", "a@b.com", "hash", "User 1")
            .await
            .unwrap();

        // Duplicate username
        let result = db
            .create_user("user1", "c@d.com", "hash", "User 1 dup")
            .await;
        assert!(result.is_err());

        // Duplicate email
        let result = db.create_user("user2", "a@b.com", "hash", "User 2").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_create_and_list_bots() {
        let db = test_db().await;

        let bot1 = db.create_bot("Bot1", "First bot", None).await.unwrap();
        assert_eq!(bot1.name, "Bot1");
        assert_eq!(bot1.description, "First bot");

        let bot2 = db.create_bot("Bot2", "Second bot", None).await.unwrap();
        assert_eq!(bot2.name, "Bot2");

        let bots = db.list_bots().await.unwrap();
        assert_eq!(bots.len(), 2);
        assert_eq!(bots[0].name, "Bot1");
        assert_eq!(bots[1].name, "Bot2");

        let fetched = db.get_bot(bot1.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "Bot1");

        let missing = db.get_bot(999).await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_bots_with_owner() {
        let db = test_db().await;

        let user = db
            .create_user("owner", "owner@test.com", "hash", "Owner")
            .await
            .unwrap();

        let bot = db
            .create_bot("OwnedBot", "desc", Some(user.id))
            .await
            .unwrap();
        assert_eq!(bot.owner_id, Some(user.id));
        assert_eq!(bot.visibility, "public");

        let user_bots = db.list_bots_by_owner(user.id).await.unwrap();
        assert_eq!(user_bots.len(), 1);
        assert_eq!(user_bots[0].name, "OwnedBot");
    }

    #[tokio::test]
    async fn test_update_bot() {
        let db = test_db().await;

        let bot = db.create_bot("Original", "desc", None).await.unwrap();
        let updated = db.update_bot(bot.id, "Updated", "new desc").await.unwrap();
        assert!(updated.is_some());
        let updated = updated.unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.description, "new desc");

        let not_found = db.update_bot(999, "X", "Y").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_delete_bot() {
        let db = test_db().await;

        let bot = db.create_bot("ToDelete", "", None).await.unwrap();
        assert!(db.delete_bot(bot.id).await.unwrap());
        assert!(!db.delete_bot(bot.id).await.unwrap());

        let bots = db.list_bots().await.unwrap();
        assert!(bots.is_empty());
    }

    #[tokio::test]
    async fn test_bot_versions() {
        let db = test_db().await;

        let bot = db.create_bot("VersionBot", "", None).await.unwrap();

        let v1 = db.create_bot_version(bot.id, "print('v1')").await.unwrap();
        assert_eq!(v1.version, 1);
        assert_eq!(v1.code, "print('v1')");

        let v2 = db.create_bot_version(bot.id, "print('v2')").await.unwrap();
        assert_eq!(v2.version, 2);

        let versions = db.list_bot_versions(bot.id).await.unwrap();
        assert_eq!(versions.len(), 2);
        assert_eq!(versions[0].version, 1);
        assert_eq!(versions[1].version, 2);

        let fetched = db.get_bot_version(bot.id, v1.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().code, "print('v1')");
    }

    #[tokio::test]
    async fn test_tournament_crud() {
        let db = test_db().await;

        let t = db.create_tournament("Tourney1", "desert").await.unwrap();
        assert_eq!(t.name, "Tourney1");
        assert_eq!(t.status, "created");
        assert_eq!(t.map, "desert");

        let tournaments = db.list_tournaments().await.unwrap();
        assert_eq!(tournaments.len(), 1);

        let fetched = db.get_tournament(t.id).await.unwrap();
        assert!(fetched.is_some());

        assert!(db.update_tournament_status(t.id, "running").await.unwrap());
        let updated = db.get_tournament(t.id).await.unwrap().unwrap();
        assert_eq!(updated.status, "running");

        assert!(!db.update_tournament_status(999, "running").await.unwrap());
    }

    #[tokio::test]
    async fn test_tournament_entries() {
        let db = test_db().await;

        let bot = db.create_bot("EntryBot", "", None).await.unwrap();
        let v = db.create_bot_version(bot.id, "code").await.unwrap();
        let t = db.create_tournament("T", "default").await.unwrap();

        let entry = db
            .add_tournament_entry(t.id, v.id, "player1")
            .await
            .unwrap();
        assert_eq!(entry.slot_name, "player1");
        assert_eq!(entry.tournament_id, t.id);

        let entries = db.list_tournament_entries(t.id).await.unwrap();
        assert_eq!(entries.len(), 1);

        assert!(db.remove_tournament_entry(entry.id).await.unwrap());
        assert!(!db.remove_tournament_entry(entry.id).await.unwrap());

        let entries = db.list_tournament_entries(t.id).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_bot_version_elo_and_stats() {
        let db = test_db().await;

        let bot = db.create_bot("EloBot", "", None).await.unwrap();
        let v = db.create_bot_version(bot.id, "code").await.unwrap();

        assert_eq!(v.elo_rating, 1500);
        assert_eq!(v.games_played, 0);

        db.update_version_elo(v.id, 1520).await.unwrap();
        db.update_version_stats(v.id, true, false, false, 100, 5, 3, 2)
            .await
            .unwrap();

        let updated = db.get_bot_version_by_id(v.id).await.unwrap().unwrap();
        assert_eq!(updated.elo_rating, 1520);
        assert_eq!(updated.elo_peak, 1520);
        assert_eq!(updated.games_played, 1);
        assert_eq!(updated.wins, 1);
        assert_eq!(updated.losses, 0);
        assert_eq!(updated.total_score, 100);
        assert_eq!(updated.creatures_spawned, 5);
        assert_eq!(updated.creatures_killed, 3);
        assert_eq!(updated.creatures_lost, 2);
    }

    #[tokio::test]
    async fn test_active_version() {
        let db = test_db().await;

        let bot = db.create_bot("ActiveBot", "", None).await.unwrap();
        assert!(bot.active_version_id.is_none());

        let v1 = db.create_bot_version(bot.id, "v1").await.unwrap();
        db.set_active_version(bot.id, v1.id).await.unwrap();

        let bot = db.get_bot(bot.id).await.unwrap().unwrap();
        assert_eq!(bot.active_version_id, Some(v1.id));

        let active = db.get_active_version(bot.id).await.unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id, v1.id);
    }

    #[tokio::test]
    async fn test_version_archiving() {
        let db = test_db().await;

        let bot = db.create_bot("ArchiveBot", "", None).await.unwrap();
        let v = db.create_bot_version(bot.id, "code").await.unwrap();
        assert_eq!(v.is_archived, 0);

        db.archive_version(v.id, true).await.unwrap();
        let archived = db.get_bot_version_by_id(v.id).await.unwrap().unwrap();
        assert_eq!(archived.is_archived, 1);

        db.archive_version(v.id, false).await.unwrap();
        let unarchived = db.get_bot_version_by_id(v.id).await.unwrap().unwrap();
        assert_eq!(unarchived.is_archived, 0);
    }

    #[tokio::test]
    async fn test_match_recording() {
        let db = test_db().await;

        let bot = db.create_bot("MatchBot", "", None).await.unwrap();
        let v1 = db.create_bot_version(bot.id, "code1").await.unwrap();
        let v2 = db.create_bot_version(bot.id, "code2").await.unwrap();

        let m = db.create_match("1v1", "random").await.unwrap();
        assert_eq!(m.format, "1v1");
        assert_eq!(m.status, "running");

        let p1 = db.add_match_participant(m.id, v1.id, 0).await.unwrap();
        let p2 = db.add_match_participant(m.id, v2.id, 1).await.unwrap();

        db.update_match_participant(p1.id, 150, Some(1), Some(1500), Some(1520), 10, 5, 3)
            .await
            .unwrap();
        db.update_match_participant(p2.id, 80, Some(2), Some(1500), Some(1480), 8, 3, 5)
            .await
            .unwrap();

        db.finish_match(m.id, Some(v1.id)).await.unwrap();

        let finished = db.get_match(m.id).await.unwrap().unwrap();
        assert_eq!(finished.status, "finished");
        assert_eq!(finished.winner_bot_version_id, Some(v1.id));

        let participants = db.get_match_participants(m.id).await.unwrap();
        assert_eq!(participants.len(), 2);
        assert_eq!(participants[0].final_score, 150);
        assert_eq!(participants[1].final_score, 80);
    }

    #[tokio::test]
    async fn test_list_recent_matches() {
        let db = test_db().await;

        // Create several matches
        let m1 = db.create_match("1v1", "random").await.unwrap();
        let m2 = db.create_match("ffa", "desert").await.unwrap();
        let m3 = db.create_match("1v1", "forest").await.unwrap();

        // List all
        let matches = db.list_recent_matches(10, 0).await.unwrap();
        assert_eq!(matches.len(), 3);
        // Should be ordered by created_at DESC (most recent first)
        // Since they are created in quick succession, IDs should be descending
        assert_eq!(matches[0].id, m3.id);
        assert_eq!(matches[1].id, m2.id);
        assert_eq!(matches[2].id, m1.id);

        // List with limit
        let limited = db.list_recent_matches(2, 0).await.unwrap();
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].id, m3.id);
        assert_eq!(limited[1].id, m2.id);

        // List with offset
        let offset = db.list_recent_matches(10, 1).await.unwrap();
        assert_eq!(offset.len(), 2);
        assert_eq!(offset[0].id, m2.id);
        assert_eq!(offset[1].id, m1.id);

        // Empty result
        let empty_db = test_db().await;
        let empty = empty_db.list_recent_matches(10, 0).await.unwrap();
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_create_team_with_versions() {
        let db = test_db().await;

        let user = db
            .create_user("teamowner", "team@test.com", "hash", "Team Owner")
            .await
            .unwrap();

        let team = db.create_team(user.id, "Alpha Squad").await.unwrap();
        assert_eq!(team.name, "Alpha Squad");
        assert_eq!(team.owner_id, user.id);

        let fetched = db.get_team(team.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().name, "Alpha Squad");

        // Create bots and versions for the team
        let bot_a = db.create_bot("BotA", "", Some(user.id)).await.unwrap();
        let bot_b = db.create_bot("BotB", "", Some(user.id)).await.unwrap();
        let va = db.create_bot_version(bot_a.id, "code_a").await.unwrap();
        let vb = db.create_bot_version(bot_b.id, "code_b").await.unwrap();

        let tv1 = db.create_team_version(team.id, va.id, vb.id).await.unwrap();
        assert_eq!(tv1.version, 1);
        assert_eq!(tv1.bot_version_a, va.id);
        assert_eq!(tv1.bot_version_b, vb.id);
        assert_eq!(tv1.elo_rating, 1500);
        assert_eq!(tv1.games_played, 0);

        let versions = db.list_team_versions(team.id).await.unwrap();
        assert_eq!(versions.len(), 1);

        let fetched_tv = db.get_team_version(team.id, tv1.id).await.unwrap();
        assert!(fetched_tv.is_some());
        assert_eq!(fetched_tv.unwrap().version, 1);
    }

    #[tokio::test]
    async fn test_list_teams_by_owner() {
        let db = test_db().await;

        let user1 = db
            .create_user("owner1", "o1@test.com", "hash", "Owner 1")
            .await
            .unwrap();
        let user2 = db
            .create_user("owner2", "o2@test.com", "hash", "Owner 2")
            .await
            .unwrap();

        db.create_team(user1.id, "Team A").await.unwrap();
        db.create_team(user1.id, "Team B").await.unwrap();
        db.create_team(user2.id, "Team C").await.unwrap();

        let teams1 = db.list_teams_by_owner(user1.id).await.unwrap();
        assert_eq!(teams1.len(), 2);
        assert_eq!(teams1[0].name, "Team A");
        assert_eq!(teams1[1].name, "Team B");

        let teams2 = db.list_teams_by_owner(user2.id).await.unwrap();
        assert_eq!(teams2.len(), 1);
        assert_eq!(teams2[0].name, "Team C");
    }

    #[tokio::test]
    async fn test_team_version_auto_increment() {
        let db = test_db().await;

        let user = db
            .create_user("tvuser", "tv@test.com", "hash", "TV User")
            .await
            .unwrap();
        let team = db.create_team(user.id, "VersionTeam").await.unwrap();

        let bot = db.create_bot("TVBot", "", Some(user.id)).await.unwrap();
        let v1 = db.create_bot_version(bot.id, "code1").await.unwrap();
        let v2 = db.create_bot_version(bot.id, "code2").await.unwrap();

        let tv1 = db.create_team_version(team.id, v1.id, v2.id).await.unwrap();
        assert_eq!(tv1.version, 1);

        let tv2 = db.create_team_version(team.id, v2.id, v1.id).await.unwrap();
        assert_eq!(tv2.version, 2);

        let tv3 = db.create_team_version(team.id, v1.id, v1.id).await.unwrap();
        assert_eq!(tv3.version, 3);

        let versions = db.list_team_versions(team.id).await.unwrap();
        assert_eq!(versions.len(), 3);
    }

    #[tokio::test]
    async fn test_update_and_delete_team() {
        let db = test_db().await;

        let user = db
            .create_user("deluser", "del@test.com", "hash", "Del User")
            .await
            .unwrap();
        let team = db.create_team(user.id, "OldName").await.unwrap();

        let updated = db
            .update_team_name(team.id, "NewName")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.name, "NewName");

        let not_found = db.update_team_name(999, "X").await.unwrap();
        assert!(not_found.is_none());

        assert!(db.delete_team(team.id).await.unwrap());
        assert!(!db.delete_team(team.id).await.unwrap());

        let gone = db.get_team(team.id).await.unwrap();
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn test_leaderboard_1v1_ordering() {
        let db = test_db().await;

        let user = db
            .create_user("lbuser", "lb@test.com", "hash", "LB User")
            .await
            .unwrap();

        // Create two bots with different elo ratings
        let bot_a = db.create_bot("BotHigh", "", Some(user.id)).await.unwrap();
        let va = db.create_bot_version(bot_a.id, "code_a").await.unwrap();
        db.update_version_elo(va.id, 1600).await.unwrap();
        db.update_version_stats(va.id, true, false, false, 200, 10, 5, 3)
            .await
            .unwrap();

        let bot_b = db.create_bot("BotLow", "", Some(user.id)).await.unwrap();
        let vb = db.create_bot_version(bot_b.id, "code_b").await.unwrap();
        db.update_version_elo(vb.id, 1520).await.unwrap();
        db.update_version_stats(vb.id, true, false, false, 100, 5, 2, 1)
            .await
            .unwrap();

        let leaderboard = db.leaderboard_1v1(50, 0).await.unwrap();
        assert_eq!(leaderboard.len(), 2);
        assert_eq!(leaderboard[0].bot_name, "BotHigh");
        assert_eq!(leaderboard[0].rating, 1600);
        assert_eq!(leaderboard[0].rank, 1);
        assert_eq!(leaderboard[0].owner_username, "lbuser");
        assert_eq!(leaderboard[1].bot_name, "BotLow");
        assert_eq!(leaderboard[1].rating, 1520);
        assert_eq!(leaderboard[1].rank, 2);
    }

    #[tokio::test]
    async fn test_leaderboard_filters_archived_and_zero_games() {
        let db = test_db().await;

        let user = db
            .create_user("filteruser", "filter@test.com", "hash", "Filter User")
            .await
            .unwrap();

        // Bot with games played (should appear)
        let bot_active = db.create_bot("ActiveBot", "", Some(user.id)).await.unwrap();
        let v_active = db.create_bot_version(bot_active.id, "code").await.unwrap();
        db.update_version_elo(v_active.id, 1550).await.unwrap();
        db.update_version_stats(v_active.id, true, false, false, 100, 5, 3, 2)
            .await
            .unwrap();

        // Archived bot with games played (should NOT appear)
        let bot_archived = db
            .create_bot("ArchivedBot", "", Some(user.id))
            .await
            .unwrap();
        let v_archived = db
            .create_bot_version(bot_archived.id, "code")
            .await
            .unwrap();
        db.update_version_elo(v_archived.id, 1700).await.unwrap();
        db.update_version_stats(v_archived.id, true, false, false, 300, 10, 8, 1)
            .await
            .unwrap();
        db.archive_version(v_archived.id, true).await.unwrap();

        // Bot with zero games (should NOT appear)
        let bot_zero = db.create_bot("ZeroBot", "", Some(user.id)).await.unwrap();
        let _v_zero = db.create_bot_version(bot_zero.id, "code").await.unwrap();

        let leaderboard = db.leaderboard_1v1(50, 0).await.unwrap();
        assert_eq!(leaderboard.len(), 1);
        assert_eq!(leaderboard[0].bot_name, "ActiveBot");

        // FFA leaderboard should behave the same way
        db.update_version_ffa_stats(v_active.id, 10).await.unwrap();
        let ffa_lb = db.leaderboard_ffa(50, 0).await.unwrap();
        assert_eq!(ffa_lb.len(), 1);
        assert_eq!(ffa_lb[0].bot_name, "ActiveBot");

        // 2v2 placeholder should return empty
        let lb_2v2 = db.leaderboard_2v2(50, 0).await.unwrap();
        assert!(lb_2v2.is_empty());
    }

    #[tokio::test]
    async fn test_api_token_crud() {
        let db = test_db().await;

        let user = db
            .create_user("tokenuser", "token@test.com", "hash", "Token User")
            .await
            .unwrap();

        // Create a token
        let token = db
            .create_api_token(user.id, "My Key", "somehash123", "bots:read,matches:read")
            .await
            .unwrap();
        assert_eq!(token.name, "My Key");
        assert_eq!(token.user_id, user.id);
        assert_eq!(token.scopes, "bots:read,matches:read");
        assert!(token.last_used_at.is_none());

        // List tokens
        let tokens = db.list_api_tokens(user.id).await.unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].name, "My Key");

        // Look up by hash
        let found = db.get_api_token_by_hash("somehash123").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, token.id);

        let not_found = db.get_api_token_by_hash("nonexistent").await.unwrap();
        assert!(not_found.is_none());

        // Update last used
        db.update_token_last_used(token.id).await.unwrap();
        let updated = db
            .get_api_token_by_hash("somehash123")
            .await
            .unwrap()
            .unwrap();
        assert!(updated.last_used_at.is_some());

        // Delete token (wrong user)
        assert!(!db.delete_api_token(token.id, 999).await.unwrap());

        // Delete token (correct user)
        assert!(db.delete_api_token(token.id, user.id).await.unwrap());
        let tokens = db.list_api_tokens(user.id).await.unwrap();
        assert!(tokens.is_empty());
    }

    #[tokio::test]
    async fn test_save_and_get_replay() {
        let db = test_db().await;

        let m = db.create_match("1v1", "random").await.unwrap();

        let data = vec![1, 2, 3, 4, 5];
        let replay = db.save_replay(m.id, &data, 42).await.unwrap();
        assert_eq!(replay.match_id, m.id);
        assert_eq!(replay.data, data);
        assert_eq!(replay.tick_count, 42);

        let fetched = db.get_replay(m.id).await.unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.match_id, m.id);
        assert_eq!(fetched.data, data);
        assert_eq!(fetched.tick_count, 42);

        // No replay for non-existent match
        let missing = db.get_replay(999).await.unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_api_token_unique_hashes() {
        let db = test_db().await;

        let user = db
            .create_user("hashuser", "hash@test.com", "hash", "Hash User")
            .await
            .unwrap();

        let t1 = db
            .create_api_token(user.id, "Key 1", "hash_aaa", "bots:read")
            .await
            .unwrap();
        let t2 = db
            .create_api_token(user.id, "Key 2", "hash_bbb", "bots:read")
            .await
            .unwrap();

        assert_ne!(t1.id, t2.id);
        assert_ne!(t1.token_hash, t2.token_hash);

        let tokens = db.list_api_tokens(user.id).await.unwrap();
        assert_eq!(tokens.len(), 2);
    }

    #[tokio::test]
    async fn test_tournament_standings() {
        let db = test_db().await;

        // Create bots and versions
        let bot_a = db.create_bot("StandingsA", "", None).await.unwrap();
        let va = db.create_bot_version(bot_a.id, "code_a").await.unwrap();
        let bot_b = db.create_bot("StandingsB", "", None).await.unwrap();
        let vb = db.create_bot_version(bot_b.id, "code_b").await.unwrap();

        // Create tournament
        let t = db
            .create_tournament("StandingsTest", "default")
            .await
            .unwrap();
        assert_eq!(t.format, "round_robin");
        assert_eq!(t.current_round, 0);
        assert_eq!(t.total_rounds, 1);

        // Add results simulating two games
        db.add_tournament_result(t.id, 0, va.id, 150, 10, 5, 3)
            .await
            .unwrap();
        db.add_tournament_result(t.id, 1, vb.id, 80, 8, 3, 5)
            .await
            .unwrap();

        let standings = db.get_tournament_standings(t.id).await.unwrap();
        assert_eq!(standings.len(), 2);
        // Ordered by total_score DESC
        assert_eq!(standings[0].bot_name, "StandingsA");
        assert_eq!(standings[0].total_score, 150);
        assert_eq!(standings[0].matches_played, 1);
        assert_eq!(standings[1].bot_name, "StandingsB");
        assert_eq!(standings[1].total_score, 80);
        assert_eq!(standings[1].matches_played, 1);
    }

    #[tokio::test]
    async fn test_soft_reset_elo() {
        let db = test_db().await;

        let bot = db.create_bot("EloResetBot", "", None).await.unwrap();

        // First version should have default 1500 Elo
        let v1 = db.create_bot_version(bot.id, "print('v1')").await.unwrap();
        assert_eq!(v1.elo_rating, 1500);
        assert_eq!(v1.elo_1v1, 1500);

        // Simulate v1 gaining Elo to 1700
        db.update_version_elo(v1.id, 1700).await.unwrap();

        // Second version should have soft reset: (1700 + 1500) / 2 = 1600
        let v2 = db.create_bot_version(bot.id, "print('v2')").await.unwrap();
        assert_eq!(v2.version, 2);
        assert_eq!(v2.elo_rating, 1600);
        assert_eq!(v2.elo_1v1, 1600);
        assert_eq!(v2.elo_peak, 1600);

        // Simulate v2 dropping to 1400
        db.update_version_elo(v2.id, 1400).await.unwrap();

        // Third version: (1400 + 1500) / 2 = 1450
        let v3 = db.create_bot_version(bot.id, "print('v3')").await.unwrap();
        assert_eq!(v3.version, 3);
        assert_eq!(v3.elo_rating, 1450);

        // Default case: new bot, first version = 1500
        let bot2 = db.create_bot("FreshBot", "", None).await.unwrap();
        let fresh_v1 = db.create_bot_version(bot2.id, "code").await.unwrap();
        assert_eq!(fresh_v1.elo_rating, 1500);
    }

    #[tokio::test]
    async fn test_tournament_update() {
        let db = test_db().await;

        let t = db.create_tournament("UpdateMe", "default").await.unwrap();
        assert_eq!(t.format, "round_robin");

        let updated = db
            .update_tournament(
                t.id,
                Some("NewName"),
                None,
                Some("single_elimination"),
                None,
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.name, "NewName");
        assert_eq!(updated.format, "single_elimination");
        assert_eq!(updated.map, "default"); // unchanged

        // Update round
        assert!(db.update_tournament_round(t.id, 2).await.unwrap());
        let fetched = db.get_tournament(t.id).await.unwrap().unwrap();
        assert_eq!(fetched.current_round, 2);

        // Not found
        let missing = db
            .update_tournament(999, Some("X"), None, None, None)
            .await
            .unwrap();
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_tournament_matches_linking() {
        let db = test_db().await;

        let t = db
            .create_tournament("LinkedTourney", "default")
            .await
            .unwrap();

        // Create some matches
        let m1 = db.create_match("1v1", "default").await.unwrap();
        let m2 = db.create_match("1v1", "default").await.unwrap();
        let m3 = db.create_match("1v1", "default").await.unwrap();

        // Link matches to tournament rounds
        let tm1 = db.add_tournament_match(t.id, m1.id, 1).await.unwrap();
        assert_eq!(tm1.tournament_id, t.id);
        assert_eq!(tm1.match_id, m1.id);
        assert_eq!(tm1.round, 1);

        let _tm2 = db.add_tournament_match(t.id, m2.id, 1).await.unwrap();
        let _tm3 = db.add_tournament_match(t.id, m3.id, 2).await.unwrap();

        // List all tournament matches
        let all = db.list_tournament_matches(t.id).await.unwrap();
        assert_eq!(all.len(), 3);

        // List by round
        let round1 = db.list_tournament_matches_by_round(t.id, 1).await.unwrap();
        assert_eq!(round1.len(), 2);
        assert_eq!(round1[0].match_id, m1.id);
        assert_eq!(round1[1].match_id, m2.id);

        let round2 = db.list_tournament_matches_by_round(t.id, 2).await.unwrap();
        assert_eq!(round2.len(), 1);
        assert_eq!(round2[0].match_id, m3.id);

        // Empty round
        let round3 = db.list_tournament_matches_by_round(t.id, 3).await.unwrap();
        assert!(round3.is_empty());

        // Get tournament for match
        let result = db.get_tournament_for_match(m1.id).await.unwrap();
        assert!(result.is_some());
        let (tid, round) = result.unwrap();
        assert_eq!(tid, t.id);
        assert_eq!(round, 1);

        let result3 = db.get_tournament_for_match(m3.id).await.unwrap();
        assert_eq!(result3, Some((t.id, 2)));

        // Non-tournament match returns None
        let m4 = db.create_match("1v1", "default").await.unwrap();
        let no_tournament = db.get_tournament_for_match(m4.id).await.unwrap();
        assert!(no_tournament.is_none());
    }

    // ── OAuth account tests ─────────────────────────────────────────

    #[tokio::test]
    async fn test_oauth_account_crud() {
        let db = test_db().await;
        let user = db
            .create_user("oauthuser", "oauth@test.com", "hash", "OAuth User")
            .await
            .unwrap();

        // Create OAuth account
        let oauth = db
            .create_oauth_account(
                user.id,
                "github",
                "gh-12345",
                Some("octocat"),
                Some("octocat@gh.com"),
            )
            .await
            .unwrap();
        assert_eq!(oauth.user_id, user.id);
        assert_eq!(oauth.provider, "github");
        assert_eq!(oauth.provider_user_id, "gh-12345");
        assert_eq!(oauth.provider_username, Some("octocat".to_string()));

        // Find by provider + provider_user_id
        let found = db.find_oauth_account("github", "gh-12345").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().user_id, user.id);

        // Not found for wrong provider
        let not_found = db.find_oauth_account("google", "gh-12345").await.unwrap();
        assert!(not_found.is_none());

        // Not found for wrong ID
        let not_found = db.find_oauth_account("github", "wrong-id").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_oauth_account_unique_constraint() {
        let db = test_db().await;
        let user = db
            .create_user("user1", "u1@test.com", "hash", "User 1")
            .await
            .unwrap();

        db.create_oauth_account(user.id, "github", "gh-1", None, None)
            .await
            .unwrap();

        // Duplicate provider + provider_user_id should fail
        let result = db
            .create_oauth_account(user.id, "github", "gh-1", None, None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_oauth_account_multiple_providers() {
        let db = test_db().await;
        let user = db
            .create_user("multi", "multi@test.com", "hash", "Multi")
            .await
            .unwrap();

        // Link GitHub
        db.create_oauth_account(user.id, "github", "gh-1", Some("multi_gh"), None)
            .await
            .unwrap();

        // Link Google (same user, different provider)
        db.create_oauth_account(user.id, "google", "goog-1", Some("multi_goog"), None)
            .await
            .unwrap();

        // Both should be findable
        assert!(db
            .find_oauth_account("github", "gh-1")
            .await
            .unwrap()
            .is_some());
        assert!(db
            .find_oauth_account("google", "goog-1")
            .await
            .unwrap()
            .is_some());
    }

    #[tokio::test]
    async fn test_create_user_oauth_no_password() {
        let db = test_db().await;
        let user = db
            .create_user_oauth(
                "oauthonly",
                "oauth@test.com",
                "OAuth User",
                Some("https://avatar.com/pic.png"),
            )
            .await
            .unwrap();

        assert_eq!(user.username, "oauthonly");
        assert_eq!(user.email, "oauth@test.com");
        assert!(user.password_hash.is_none()); // no password for OAuth users
        assert_eq!(
            user.avatar_url,
            Some("https://avatar.com/pic.png".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_user_by_email() {
        let db = test_db().await;
        db.create_user("emailuser", "find@me.com", "hash", "Find Me")
            .await
            .unwrap();

        let found = db.get_user_by_email("find@me.com").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().username, "emailuser");

        let not_found = db.get_user_by_email("missing@nowhere.com").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_update_user_avatar() {
        let db = test_db().await;
        let user = db
            .create_user("avuser", "av@test.com", "hash", "Avatar User")
            .await
            .unwrap();
        assert!(user.avatar_url.is_none());

        db.update_user_avatar(user.id, "https://example.com/new.png")
            .await
            .unwrap();

        let updated = db.get_user(user.id).await.unwrap().unwrap();
        assert_eq!(
            updated.avatar_url,
            Some("https://example.com/new.png".to_string())
        );
    }

    #[tokio::test]
    async fn test_username_exists() {
        let db = test_db().await;
        db.create_user("exists", "e@test.com", "hash", "Exists")
            .await
            .unwrap();

        assert!(db.username_exists("exists").await.unwrap());
        assert!(!db.username_exists("doesnotexist").await.unwrap());
    }

    #[tokio::test]
    async fn test_oauth_cascade_delete() {
        let db = test_db().await;
        let user = db
            .create_user("delme", "del@test.com", "hash", "Delete Me")
            .await
            .unwrap();

        db.create_oauth_account(user.id, "github", "gh-del", None, None)
            .await
            .unwrap();

        // Verify it exists
        assert!(db
            .find_oauth_account("github", "gh-del")
            .await
            .unwrap()
            .is_some());

        // Delete the user - OAuth account should cascade
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user.id)
            .execute(&db.pool)
            .await
            .unwrap();

        // OAuth account should be gone
        assert!(db
            .find_oauth_account("github", "gh-del")
            .await
            .unwrap()
            .is_none());
    }
}
