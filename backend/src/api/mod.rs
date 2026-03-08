// HTTP API routes (bot CRUD, game control, etc.)

pub mod ws;

use axum::{
    extract::{Json, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use std::path::PathBuf;

use crate::auth::{AuthUser, OptionalAuthUser};
use crate::db::Database;
use crate::engine::server::{self, GameResult, GameServer, PlayerEntry};
use crate::engine::world::World;
use crate::metrics;
use crate::queue::run_game_completion;
use crate::rate_limit::{RateLimitType, RateLimiter};
use crate::tournament::{
    generate_round_robin_pairings, generate_single_elimination_bracket, generate_swiss_pairings,
    total_rounds, TournamentFormat,
};

// ── Request types ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateBotRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateBotRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateBotVersionRequest {
    pub code: String,
}

#[derive(Deserialize)]
pub struct CreateTournamentRequest {
    pub name: String,
    pub map: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateTournamentRequest {
    pub name: Option<String>,
    pub map: Option<String>,
    pub format: Option<String>,
    pub config: Option<String>,
}

#[derive(Deserialize)]
pub struct AddTournamentEntryRequest {
    pub bot_version_id: i64,
    pub slot_name: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct MapParamsRequest {
    pub width: Option<usize>,
    pub height: Option<usize>,
    pub num_food_spots: Option<usize>,
}

#[derive(Deserialize)]
pub struct StartGameRequest {
    pub players: Vec<StartGamePlayer>,
    pub map: Option<String>,
    pub headless: Option<bool>,
    pub map_params: Option<MapParamsRequest>,
}

#[derive(Deserialize)]
pub struct StartGamePlayer {
    pub bot_version_id: i64,
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct SetActiveVersionRequest {
    pub version_id: i64,
}

#[derive(Deserialize)]
pub struct UpdateBotVersionRequest {
    pub is_archived: Option<bool>,
}

#[derive(Deserialize)]
pub struct ChallengeRequest {
    pub bot_version_id: i64,
    pub opponent_bot_version_id: i64,
    pub format: Option<String>,
    pub headless: Option<bool>,
    pub map: Option<String>,
}

#[derive(Deserialize)]
pub struct PaginationParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct MatchListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub bot_id: Option<i64>,
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub sort: Option<String>,
    pub status: Option<String>,
    pub map: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct UpdateTeamRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct CreateTeamVersionRequest {
    pub bot_version_a: i64,
    pub bot_version_b: i64,
}

#[derive(Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Option<String>,
}

#[derive(Deserialize)]
pub struct ValidateLuaRequest {
    pub code: String,
}

#[derive(Deserialize)]
pub struct CreateFeedbackRequest {
    pub category: String,
    pub description: String,
}

// ── Shared application state ─────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub game_server: Arc<GameServer>,
    pub maps_dir: PathBuf,
    pub rate_limiter: RateLimiter,
}

// ── Error helper ──────────────────────────────────────────────────────

fn json_error(status: StatusCode, msg: &str) -> impl IntoResponse {
    (status, Json(json!({ "error": msg })))
}

fn internal_error(e: sqlx::Error) -> impl IntoResponse {
    tracing::error!("Database error: {e}");
    json_error(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
}

// ── Router ────────────────────────────────────────────────────────────

pub fn router(
    db: Arc<Database>,
    game_server: Arc<GameServer>,
    rate_limiter: RateLimiter,
    maps_dir: PathBuf,
) -> Router {
    let state = AppState {
        db,
        game_server,
        maps_dir,
        rate_limiter,
    };

    Router::new()
        // Maps
        .route("/api/maps", get(list_maps))
        // Bots
        .route("/api/bots", get(list_bots).post(create_bot))
        .route(
            "/api/bots/{id}",
            get(get_bot).put(update_bot).delete(delete_bot),
        )
        // Bot versions
        .route(
            "/api/bots/{id}/versions",
            get(list_bot_versions).post(create_bot_version),
        )
        .route(
            "/api/bots/{bot_id}/versions/{version_id}",
            get(get_bot_version).put(update_bot_version),
        )
        .route("/api/bots/{id}/active-version", put(set_active_version))
        .route("/api/bots/{id}/stats", get(get_bot_stats))
        // Matches
        .route("/api/matches", get(list_matches))
        .route("/api/matches/mine", get(list_my_matches))
        .route("/api/matches/challenge", post(create_challenge))
        .route("/api/matches/{id}", get(get_match))
        .route("/api/matches/{id}/replay", get(get_match_replay))
        // Queue
        .route("/api/queue/status", get(queue_status))
        // Tournaments
        .route(
            "/api/tournaments",
            get(list_tournaments).post(create_tournament),
        )
        .route(
            "/api/tournaments/{id}",
            get(get_tournament).put(update_tournament),
        )
        .route(
            "/api/tournaments/{id}/standings",
            get(get_tournament_standings),
        )
        // Tournament entries
        .route(
            "/api/tournaments/{id}/entries",
            get(list_tournament_entries).post(add_tournament_entry),
        )
        .route(
            "/api/tournaments/{id}/entries/{entry_id}",
            delete(remove_tournament_entry),
        )
        // Tournament results
        .route("/api/tournaments/{id}/results", get(get_tournament_results))
        .route("/api/tournaments/{id}/matches", get(get_tournament_matches))
        // Tournament run
        .route("/api/tournaments/{id}/run", post(run_tournament))
        // Leaderboards
        .route("/api/leaderboards/1v1", get(leaderboard_1v1))
        .route("/api/leaderboards/ffa", get(leaderboard_ffa))
        .route("/api/leaderboards/2v2", get(leaderboard_2v2))
        // Teams
        .route("/api/teams", get(list_teams).post(create_team))
        .route(
            "/api/teams/{id}",
            get(get_team).put(update_team).delete(delete_team),
        )
        .route(
            "/api/teams/{id}/versions",
            get(list_team_versions).post(create_team_version),
        )
        // Lua validation
        .route("/api/validate-lua", post(validate_lua))
        // Active games
        .route("/api/games/active", get(list_active_games))
        // Game control
        .route("/api/game/start", post(start_game))
        .route("/api/game/status", get(game_status))
        .route("/api/game/stop", post(stop_game))
        // Notifications
        .route("/api/notifications", get(list_notifications))
        .route("/api/notifications/{id}/read", post(mark_notification_read))
        // API keys
        .route("/api/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api/api-keys/{id}", delete(delete_api_key))
        // Documentation
        .route("/api/docs/lua-api", get(get_lua_api_docs))
        .route("/llms.txt", get(get_llms_txt))
        .route("/llms-full.txt", get(get_llms_full_txt))
        // Feedback
        .route("/api/feedback", get(list_feedback).post(create_feedback))
        // WebSocket
        .route("/ws/game", get(ws::ws_game))
        .with_state(state)
}

// ── Bot handlers ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ListBotsParams {
    pub all: Option<bool>,
}

async fn list_bots(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<ListBotsParams>,
) -> impl IntoResponse {
    let show_all = params.all.unwrap_or(false);
    let result = if !show_all {
        state.db.list_bot_summaries_by_owner(auth.0.sub).await
    } else {
        state.db.list_bot_summaries().await
    };

    match result {
        Ok(bots) => (StatusCode::OK, Json(json!(bots))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn create_bot(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateBotRequest>,
) -> impl IntoResponse {
    if req.name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name is required").into_response();
    }
    let description = req.description.unwrap_or_default();
    let owner_id = Some(auth.0.sub);
    match state.db.create_bot(&req.name, &description, owner_id).await {
        Ok(bot) => (StatusCode::CREATED, Json(json!(bot))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn get_bot(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    match state.db.get_bot(id).await {
        Ok(Some(bot)) => (StatusCode::OK, Json(json!(bot))).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Bot not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn update_bot(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateBotRequest>,
) -> impl IntoResponse {
    if req.name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name is required").into_response();
    }
    // Check ownership
    match state.db.get_bot(id).await {
        Ok(Some(bot)) => {
            if let Some(owner_id) = bot.owner_id {
                if owner_id != auth.0.sub {
                    return json_error(StatusCode::FORBIDDEN, "You do not own this bot")
                        .into_response();
                }
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Bot not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
    }
    let description = req.description.unwrap_or_default();
    match state.db.update_bot(id, &req.name, &description).await {
        Ok(Some(bot)) => (StatusCode::OK, Json(json!(bot))).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Bot not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn delete_bot(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    // Check ownership
    match state.db.get_bot(id).await {
        Ok(Some(bot)) => {
            if let Some(owner_id) = bot.owner_id {
                if owner_id != auth.0.sub {
                    return json_error(StatusCode::FORBIDDEN, "You do not own this bot")
                        .into_response();
                }
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Bot not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
    }
    match state.db.delete_bot(id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "Bot not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

// ── Bot version handlers ──────────────────────────────────────────────

async fn list_bot_versions(
    State(state): State<AppState>,
    Path(bot_id): Path<i64>,
) -> impl IntoResponse {
    // Check bot exists
    match state.db.get_bot(bot_id).await {
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Bot not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
        Ok(Some(_)) => {}
    }
    match state.db.list_bot_versions(bot_id).await {
        Ok(versions) => (StatusCode::OK, Json(json!(versions))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

const MAX_BOT_CODE_SIZE: usize = 64 * 1024; // 64KB

async fn create_bot_version(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(bot_id): Path<i64>,
    Json(req): Json<CreateBotVersionRequest>,
) -> impl IntoResponse {
    // Enforce code size limit
    if req.code.len() > MAX_BOT_CODE_SIZE {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("Bot code exceeds maximum size of {}KB", MAX_BOT_CODE_SIZE / 1024)})),
        )
            .into_response();
    }

    // Check bot exists and ownership
    match state.db.get_bot(bot_id).await {
        Ok(Some(bot)) => {
            if let Some(owner_id) = bot.owner_id {
                if owner_id != auth.0.sub {
                    return json_error(StatusCode::FORBIDDEN, "You do not own this bot")
                        .into_response();
                }
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Bot not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
    }
    match state.db.create_bot_version(bot_id, &req.code).await {
        Ok(version) => {
            metrics::BOT_SUBMISSIONS_TOTAL.inc();
            (StatusCode::CREATED, Json(json!(version))).into_response()
        }
        Err(e) => internal_error(e).into_response(),
    }
}

async fn get_bot_version(
    State(state): State<AppState>,
    Path((bot_id, version_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    match state.db.get_bot_version(bot_id, version_id).await {
        Ok(Some(version)) => (StatusCode::OK, Json(json!(version))).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Bot version not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

// ── Bot version management handlers ──────────────────────────────────

async fn update_bot_version(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((_bot_id, version_id)): Path<(i64, i64)>,
    Json(req): Json<UpdateBotVersionRequest>,
) -> impl IntoResponse {
    // Check ownership via the version's bot
    match state.db.get_bot_version_by_id(version_id).await {
        Ok(Some(version)) => match state.db.get_bot(version.bot_id).await {
            Ok(Some(bot)) => {
                if let Some(owner_id) = bot.owner_id {
                    if owner_id != auth.0.sub {
                        return json_error(StatusCode::FORBIDDEN, "You do not own this bot")
                            .into_response();
                    }
                }
            }
            Ok(None) => return json_error(StatusCode::NOT_FOUND, "Bot not found").into_response(),
            Err(e) => return internal_error(e).into_response(),
        },
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Version not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
    }
    if let Some(archived) = req.is_archived {
        match state.db.archive_version(version_id, archived).await {
            Ok(true) => {}
            Ok(false) => {
                return json_error(StatusCode::NOT_FOUND, "Version not found").into_response()
            }
            Err(e) => return internal_error(e).into_response(),
        }
    }
    match state.db.get_bot_version_by_id(version_id).await {
        Ok(Some(v)) => (StatusCode::OK, Json(json!(v))).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Version not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn set_active_version(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(bot_id): Path<i64>,
    Json(req): Json<SetActiveVersionRequest>,
) -> impl IntoResponse {
    // Check ownership
    match state.db.get_bot(bot_id).await {
        Ok(Some(bot)) => {
            if let Some(owner_id) = bot.owner_id {
                if owner_id != auth.0.sub {
                    return json_error(StatusCode::FORBIDDEN, "You do not own this bot")
                        .into_response();
                }
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Bot not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
    }
    // Verify the version belongs to this bot
    match state.db.get_bot_version(bot_id, req.version_id).await {
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Version not found for this bot")
                .into_response()
        }
        Err(e) => return internal_error(e).into_response(),
        Ok(Some(_)) => {}
    }
    match state.db.set_active_version(bot_id, req.version_id).await {
        Ok(true) => match state.db.get_bot(bot_id).await {
            Ok(Some(bot)) => (StatusCode::OK, Json(json!(bot))).into_response(),
            _ => StatusCode::OK.into_response(),
        },
        Ok(false) => json_error(StatusCode::NOT_FOUND, "Bot not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn get_bot_stats(
    State(state): State<AppState>,
    Path(bot_id): Path<i64>,
) -> impl IntoResponse {
    match state.db.list_bot_versions(bot_id).await {
        Ok(versions) => {
            let stats: Vec<serde_json::Value> = versions
                .iter()
                .map(|v| {
                    json!({
                        "version_id": v.id,
                        "version": v.version,
                        "elo_rating": v.elo_rating,
                        "elo_1v1": v.elo_1v1,
                        "elo_peak": v.elo_peak,
                        "games_played": v.games_played,
                        "wins": v.wins,
                        "losses": v.losses,
                        "draws": v.draws,
                        "win_rate": if v.games_played > 0 {
                            v.wins as f64 / v.games_played as f64
                        } else {
                            0.0
                        },
                        "ffa_placement_points": v.ffa_placement_points,
                        "ffa_games": v.ffa_games,
                        "avg_ffa_placement": if v.ffa_games > 0 {
                            v.ffa_placement_points as f64 / v.ffa_games as f64
                        } else {
                            0.0
                        },
                        "creatures_spawned": v.creatures_spawned,
                        "creatures_killed": v.creatures_killed,
                        "creatures_lost": v.creatures_lost,
                        "total_score": v.total_score,
                        "avg_score": if v.games_played > 0 {
                            v.total_score as f64 / v.games_played as f64
                        } else {
                            0.0
                        },
                        "is_archived": v.is_archived != 0,
                        "is_faulty": v.is_faulty != 0,
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!(stats))).into_response()
        }
        Err(e) => internal_error(e).into_response(),
    }
}

async fn get_match(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    let m = match state.db.get_match(id).await {
        Ok(Some(m)) => m,
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Match not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
    };
    let participants = match state.db.get_match_participants(id).await {
        Ok(p) => p,
        Err(e) => return internal_error(e).into_response(),
    };
    (
        StatusCode::OK,
        Json(json!({
            "match": m,
            "participants": participants,
        })),
    )
        .into_response()
}

// ── Replay handler ────────────────────────────────────────────────────

async fn get_match_replay(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    // Check match exists
    match state.db.get_match(id).await {
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Match not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
        Ok(Some(_)) => {}
    }

    let replay = match state.db.get_replay(id).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Replay not found for this match")
                .into_response()
        }
        Err(e) => return internal_error(e).into_response(),
    };

    // Decompress the replay data
    let json_str = match crate::replay::decompress_replay(&replay.data) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to decompress replay: {e}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to decompress replay",
            )
            .into_response();
        }
    };

    // Parse the messages array from the decompressed JSON
    let messages: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Failed to parse replay JSON: {e}");
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to parse replay data",
            )
            .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "match_id": replay.match_id,
            "tick_count": replay.tick_count,
            "messages": messages,
        })),
    )
        .into_response()
}

// ── Tournament handlers ───────────────────────────────────────────────

async fn list_tournaments(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.list_tournaments().await {
        Ok(tournaments) => (StatusCode::OK, Json(json!(tournaments))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn create_tournament(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<CreateTournamentRequest>,
) -> impl IntoResponse {
    if req.name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name is required").into_response();
    }
    let map = req.map.unwrap_or_else(|| "random".to_string());
    match state.db.create_tournament(&req.name, &map).await {
        Ok(tournament) => (StatusCode::CREATED, Json(json!(tournament))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn get_tournament(State(state): State<AppState>, Path(id): Path<i64>) -> impl IntoResponse {
    match state.db.get_tournament(id).await {
        Ok(Some(tournament)) => (StatusCode::OK, Json(json!(tournament))).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Tournament not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn update_tournament(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTournamentRequest>,
) -> impl IntoResponse {
    // Validate format if provided
    if let Some(ref fmt) = req.format {
        if crate::tournament::TournamentFormat::from_str_name(fmt).is_none() {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Invalid format. Use 'round_robin', 'single_elimination', or 'swiss_N'",
            )
            .into_response();
        }
    }
    match state
        .db
        .update_tournament(
            id,
            req.name.as_deref(),
            req.map.as_deref(),
            req.format.as_deref(),
            req.config.as_deref(),
        )
        .await
    {
        Ok(Some(tournament)) => (StatusCode::OK, Json(json!(tournament))).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Tournament not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn get_tournament_standings(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.db.get_tournament(id).await {
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Tournament not found").into_response()
        }
        Err(e) => return internal_error(e).into_response(),
        Ok(Some(_)) => {}
    }
    match state.db.get_tournament_standings(id).await {
        Ok(standings) => (StatusCode::OK, Json(json!(standings))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

// ── Tournament entry handlers ─────────────────────────────────────────

async fn list_tournament_entries(
    State(state): State<AppState>,
    Path(tournament_id): Path<i64>,
) -> impl IntoResponse {
    match state.db.get_tournament(tournament_id).await {
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Tournament not found").into_response()
        }
        Err(e) => return internal_error(e).into_response(),
        Ok(Some(_)) => {}
    }
    match state.db.list_tournament_entries(tournament_id).await {
        Ok(entries) => (StatusCode::OK, Json(json!(entries))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn add_tournament_entry(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(tournament_id): Path<i64>,
    Json(req): Json<AddTournamentEntryRequest>,
) -> impl IntoResponse {
    match state.db.get_tournament(tournament_id).await {
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Tournament not found").into_response()
        }
        Err(e) => return internal_error(e).into_response(),
        Ok(Some(_)) => {}
    }
    let slot_name = req.slot_name.unwrap_or_default();
    match state
        .db
        .add_tournament_entry(tournament_id, req.bot_version_id, &slot_name)
        .await
    {
        Ok(entry) => (StatusCode::CREATED, Json(json!(entry))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn remove_tournament_entry(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path((_tournament_id, entry_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    match state.db.remove_tournament_entry(entry_id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => {
            json_error(StatusCode::NOT_FOUND, "Tournament entry not found").into_response()
        }
        Err(e) => internal_error(e).into_response(),
    }
}

// ── Tournament result handlers ────────────────────────────────────────

async fn get_tournament_results(
    State(state): State<AppState>,
    Path(tournament_id): Path<i64>,
) -> impl IntoResponse {
    match state.db.get_tournament(tournament_id).await {
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Tournament not found").into_response()
        }
        Err(e) => return internal_error(e).into_response(),
        Ok(Some(_)) => {}
    }
    match state.db.get_tournament_results(tournament_id).await {
        Ok(results) => (StatusCode::OK, Json(json!(results))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

// ── Tournament matches detail handler ─────────────────────────────

async fn get_tournament_matches(
    State(state): State<AppState>,
    Path(tournament_id): Path<i64>,
) -> impl IntoResponse {
    match state.db.get_tournament(tournament_id).await {
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Tournament not found").into_response()
        }
        Err(e) => return internal_error(e).into_response(),
        Ok(Some(_)) => {}
    }
    match state.db.get_tournament_matches_detail(tournament_id).await {
        Ok(details) => {
            // Group by round
            let mut rounds_map: std::collections::BTreeMap<
                i32,
                Vec<&crate::db::TournamentMatchDetail>,
            > = std::collections::BTreeMap::new();
            for d in &details {
                rounds_map.entry(d.round).or_default().push(d);
            }
            let rounds: Vec<serde_json::Value> = rounds_map
                .into_iter()
                .map(|(round, matches)| {
                    json!({
                        "round": round,
                        "matches": matches,
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!({ "rounds": rounds }))).into_response()
        }
        Err(e) => internal_error(e).into_response(),
    }
}

// ── Tournament run handler ───────────────────────────────────────────

async fn run_tournament(
    State(state): State<AppState>,
    _auth: AuthUser,
    Path(tournament_id): Path<i64>,
) -> impl IntoResponse {
    // Check tournament exists
    let tournament = match state.db.get_tournament(tournament_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return json_error(StatusCode::NOT_FOUND, "Tournament not found").into_response()
        }
        Err(e) => return internal_error(e).into_response(),
    };

    // Load tournament entries
    let entries = match state.db.list_tournament_entries(tournament_id).await {
        Ok(e) => e,
        Err(e) => return internal_error(e).into_response(),
    };

    if entries.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "Tournament has no entries").into_response();
    }

    if entries.len() < 2 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Tournament needs at least 2 entries",
        )
        .into_response();
    }

    // Parse tournament format
    let format =
        TournamentFormat::from_str_name(&tournament.format).unwrap_or(TournamentFormat::RoundRobin);

    let version_ids: Vec<i64> = entries.iter().map(|e| e.bot_version_id).collect();

    // Generate pairings for current round
    let pairings = match &format {
        TournamentFormat::RoundRobin => generate_round_robin_pairings(&version_ids),
        TournamentFormat::SingleElimination => generate_single_elimination_bracket(&version_ids),
        TournamentFormat::Swiss { .. } => {
            // First round: no standings yet
            generate_swiss_pairings(&version_ids, &[], 0)
        }
    };

    let num_rounds = total_rounds(&format, version_ids.len());
    let _ = state
        .db
        .update_tournament(
            tournament_id,
            None,
            None,
            None,
            Some(&serde_json::json!({"total_rounds": num_rounds}).to_string()),
        )
        .await;

    // Create and queue matches for each pairing
    let mut match_ids = Vec::new();
    for (vid_a, vid_b) in &pairings {
        // Verify both bot versions exist
        let va = state.db.get_bot_version_by_id(*vid_a).await;
        let vb = state.db.get_bot_version_by_id(*vid_b).await;
        match (va, vb) {
            (Ok(Some(_)), Ok(Some(_))) => {}
            _ => continue,
        };

        // Create match in DB
        let m = match state.db.create_match("1v1", &tournament.map).await {
            Ok(m) => m,
            Err(_) => continue,
        };
        let _ = state.db.add_match_participant(m.id, *vid_a, 0).await;
        let _ = state.db.add_match_participant(m.id, *vid_b, 1).await;
        let _ = state.db.add_tournament_match(tournament_id, m.id, 1).await;

        // Queue the match via DB — tournament matches get priority 10
        if let Err(e) = state
            .db
            .enqueue_game(m.id, Some(&tournament.map), 10, None)
            .await
        {
            tracing::error!("Failed to enqueue tournament match {}: {e}", m.id);
        }

        match_ids.push(m.id);
    }

    // Update tournament status
    let _ = state
        .db
        .update_tournament_status(tournament_id, "running")
        .await;
    let _ = state.db.update_tournament_round(tournament_id, 1).await;

    (
        StatusCode::OK,
        Json(json!({
            "status": "running",
            "tournament_id": tournament_id,
            "matches_queued": match_ids.len(),
            "match_ids": match_ids,
            "format": tournament.format,
            "total_rounds": num_rounds,
        })),
    )
        .into_response()
}

// ── Map handlers ─────────────────────────────────────────────────────

async fn list_maps(State(state): State<AppState>) -> impl IntoResponse {
    let mut maps = server::list_maps(&state.maps_dir);
    // Prepend pseudo-entries for random options
    maps.insert(
        0,
        server::MapInfo {
            name: "random_pool".to_string(),
            width: 0,
            height: 0,
            description: "Random from pool".to_string(),
        },
    );
    maps.insert(
        0,
        server::MapInfo {
            name: "random".to_string(),
            width: 0,
            height: 0,
            description: "Random generated".to_string(),
        },
    );
    (StatusCode::OK, Json(json!(maps))).into_response()
}

/// Resolve an optional map name to a World.
pub fn resolve_map(
    maps_dir: &std::path::Path,
    map: &Option<String>,
    map_params: Option<&MapParamsRequest>,
) -> Result<World, String> {
    use crate::engine::world::RandomMapParams;
    use rand::seq::SliceRandom;
    match map.as_deref() {
        None | Some("random") | Some("default") => {
            let mut params = RandomMapParams::default();
            if let Some(mp) = map_params {
                if let Some(w) = mp.width {
                    params.width = w.clamp(20, 150);
                }
                if let Some(h) = mp.height {
                    params.height = h.clamp(20, 150);
                }
                if let Some(f) = mp.num_food_spots {
                    params.num_food_spots = f.clamp(1, 200);
                }
            }
            Ok(World::generate_random(params))
        }
        Some("random_pool") => {
            let available = server::list_maps(maps_dir);
            if available.is_empty() {
                // Fall back to generated if no map files exist
                Ok(World::generate_random(RandomMapParams::default()))
            } else {
                let mut rng = rand::thread_rng();
                let chosen = available.choose(&mut rng).unwrap();
                server::load_map(maps_dir, &chosen.name)
            }
        }
        Some(name) => server::load_map(maps_dir, name),
    }
}

// ── Leaderboard handlers ─────────────────────────────────────────────

async fn leaderboard_1v1(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0).max(0);
    match state.db.leaderboard_1v1(limit, offset).await {
        Ok(entries) => (StatusCode::OK, Json(json!(entries))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn leaderboard_ffa(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0).max(0);
    match state.db.leaderboard_ffa(limit, offset).await {
        Ok(entries) => (StatusCode::OK, Json(json!(entries))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn leaderboard_2v2(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0).max(0);
    match state.db.leaderboard_2v2(limit, offset).await {
        Ok(entries) => (StatusCode::OK, Json(json!(entries))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

// ── Lua validation handler ────────────────────────────────────────────

async fn validate_lua(_auth: AuthUser, Json(req): Json<ValidateLuaRequest>) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || {
        let lua = mlua::Lua::new();
        match lua.load(&req.code).set_name("user_bot").into_function() {
            Ok(_) => serde_json::json!({ "valid": true }),
            Err(e) => {
                metrics::BOT_VALIDATION_FAILURES_TOTAL.inc();
                serde_json::json!({ "valid": false, "error": e.to_string() })
            }
        }
    })
    .await;

    match result {
        Ok(json) => (StatusCode::OK, Json(json)).into_response(),
        Err(_) => {
            metrics::BOT_VALIDATION_FAILURES_TOTAL.inc();
            (
                StatusCode::OK,
                Json(json!({ "valid": false, "error": "Validation timed out" })),
            )
                .into_response()
        }
    }
}

// ── Active games handler ─────────────────────────────────────────────

async fn list_active_games(State(state): State<AppState>) -> impl IntoResponse {
    let mut active_games = Vec::new();
    if let Some(info) = state.game_server.active_game_info() {
        active_games.push(json!(info));
    }
    (StatusCode::OK, Json(json!(active_games))).into_response()
}

// ── Notification handlers ────────────────────────────────────────────

async fn list_notifications(State(state): State<AppState>, auth: AuthUser) -> impl IntoResponse {
    match state.db.list_recent_notifications(auth.0.sub, 20).await {
        Ok(notifications) => {
            let unread_count = state
                .db
                .unread_notification_count(auth.0.sub)
                .await
                .unwrap_or(0);
            (
                StatusCode::OK,
                Json(json!({
                    "notifications": notifications,
                    "unread_count": unread_count,
                })),
            )
                .into_response()
        }
        Err(e) => internal_error(e).into_response(),
    }
}

async fn mark_notification_read(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.db.mark_notification_read(id, auth.0.sub).await {
        Ok(true) => (StatusCode::OK, Json(json!({ "ok": true }))).into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "Notification not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

// ── Game control handlers ────────────────────────────────────────────

async fn start_game(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<StartGameRequest>,
) -> impl IntoResponse {
    let headless_early = req.headless.unwrap_or(false);
    if !headless_early && state.game_server.is_running() {
        return json_error(StatusCode::CONFLICT, "A game is already running").into_response();
    }

    if req.players.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "At least one player is required")
            .into_response();
    }

    let mut players = Vec::new();
    let mut bot_version_ids = Vec::new();
    for (i, p) in req.players.iter().enumerate() {
        let version = match state.db.get_bot_version_by_id(p.bot_version_id).await {
            Ok(Some(v)) => v,
            Ok(None) => {
                return json_error(
                    StatusCode::NOT_FOUND,
                    &format!("Bot version {} not found", p.bot_version_id),
                )
                .into_response();
            }
            Err(e) => return internal_error(e).into_response(),
        };

        let name = if let Some(n) = p.name.clone() {
            n
        } else {
            // Look up the bot name from the DB
            match state.db.get_bot(version.bot_id).await {
                Ok(Some(b)) => b.name,
                _ => format!("Player {}", i + 1),
            }
        };

        bot_version_ids.push(p.bot_version_id);
        players.push(PlayerEntry {
            name,
            code: version.code,
        });
    }

    let world = match resolve_map(&state.maps_dir, &req.map, req.map_params.as_ref()) {
        Ok(w) => w,
        Err(e) => {
            return json_error(StatusCode::BAD_REQUEST, &format!("Invalid map: {}", e))
                .into_response()
        }
    };

    // Create match record in DB
    let map_name = req.map.clone().unwrap_or_else(|| "random".to_string());
    let format = if players.len() == 2 { "1v1" } else { "ffa" };
    let m = match state.db.create_match(format, &map_name).await {
        Ok(m) => m,
        Err(e) => return internal_error(e).into_response(),
    };

    // Add participants
    for (slot, &bvid) in bot_version_ids.iter().enumerate() {
        if let Err(e) = state
            .db
            .add_match_participant(m.id, bvid, slot as i32)
            .await
        {
            return internal_error(e).into_response();
        }
    }

    let headless = req.headless.unwrap_or(false);

    if headless {
        // Queue headless game via DB
        let map_params_json = req
            .map_params
            .as_ref()
            .and_then(|mp| serde_json::to_string(mp).ok());
        if let Err(e) = state
            .db
            .enqueue_game(m.id, req.map.as_deref(), 0, map_params_json.as_deref())
            .await
        {
            tracing::error!("Failed to enqueue headless game: {e}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to queue game")
                .into_response();
        }

        return (
            StatusCode::OK,
            Json(json!({
                "status": "queued",
                "match_id": m.id,
                "message": "Headless game queued."
            })),
        )
            .into_response();
    }

    // Build completion callback for Elo, replay, and match finishing
    let on_complete = build_game_completion_callback(
        state.db.clone(),
        m.id,
        bot_version_ids.clone(),
        format.to_string(),
    );

    match state.game_server.start_game_with_callback(
        world,
        players,
        None,
        Some(m.id),
        bot_version_ids,
        false,
        Some(on_complete),
    ) {
        Ok(()) => {
            state.game_server.set_game_map(&map_name);
            (
                StatusCode::OK,
                Json(json!({
                    "status": "running",
                    "match_id": m.id,
                    "message": "Game started. Connect to /ws/game for live updates."
                })),
            )
                .into_response()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

async fn game_status(State(state): State<AppState>) -> impl IntoResponse {
    let running = state.game_server.is_running();
    (
        StatusCode::OK,
        Json(json!({
            "running": running,
        })),
    )
        .into_response()
}

async fn stop_game(State(state): State<AppState>, _auth: AuthUser) -> impl IntoResponse {
    if !state.game_server.is_running() {
        return json_error(StatusCode::BAD_REQUEST, "No game is running").into_response();
    }
    state.game_server.stop_game();
    (StatusCode::OK, Json(json!({ "status": "stopping" }))).into_response()
}

// ── Match list handler ────────────────────────────────────────────────

async fn list_matches(
    State(state): State<AppState>,
    Query(params): Query<MatchListParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0).max(0);
    let sort = params.sort.as_deref().unwrap_or("newest");
    let matches = match state
        .db
        .list_matches_filtered(
            limit,
            offset,
            params.bot_id,
            params.user_id,
            params.username.as_deref(),
            params.status.as_deref(),
            params.map.as_deref(),
            sort,
        )
        .await
    {
        Ok(m) => m,
        Err(e) => return internal_error(e).into_response(),
    };

    // Fetch player names for these matches
    let match_ids: Vec<i64> = matches.iter().map(|m| m.id).collect();
    let player_names = match state.db.get_match_player_names(&match_ids).await {
        Ok(names) => names,
        Err(e) => return internal_error(e).into_response(),
    };

    // Group player names by match_id
    let mut players_map: std::collections::HashMap<i64, Vec<String>> =
        std::collections::HashMap::new();
    for (mid, name) in player_names {
        players_map.entry(mid).or_default().push(name);
    }

    // Build response with players field
    let result: Vec<serde_json::Value> = matches
        .iter()
        .map(|m| {
            let mut v = serde_json::to_value(m).unwrap();
            v["players"] = json!(players_map.get(&m.id).unwrap_or(&vec![]));
            v
        })
        .collect();

    (StatusCode::OK, Json(json!(result))).into_response()
}

// ── My matches handler ───────────────────────────────────────────────

async fn list_my_matches(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0).max(0);
    match state.db.list_user_matches(auth.0.sub, limit, offset).await {
        Ok(matches) => (StatusCode::OK, Json(json!(matches))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

// ── Challenge handler ────────────────────────────────────────────────

async fn create_challenge(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<ChallengeRequest>,
) -> impl IntoResponse {
    if !crate::auth::has_scope(&auth.0, "matches:write") {
        return json_error(StatusCode::FORBIDDEN, "Insufficient API token scope").into_response();
    }
    let user_id = auth.0.sub;
    let headless = req.headless.unwrap_or(false);
    let format = req.format.clone().unwrap_or_else(|| "1v1".to_string());

    if format != "1v1" && format != "ffa" {
        return json_error(StatusCode::BAD_REQUEST, "format must be '1v1' or 'ffa'")
            .into_response();
    }

    // Validate both bot versions exist
    let version_a = match state.db.get_bot_version_by_id(req.bot_version_id).await {
        Ok(Some(v)) => v,
        Ok(None) => {
            return json_error(
                StatusCode::NOT_FOUND,
                &format!("Bot version {} not found", req.bot_version_id),
            )
            .into_response();
        }
        Err(e) => return internal_error(e).into_response(),
    };

    let version_b = match state
        .db
        .get_bot_version_by_id(req.opponent_bot_version_id)
        .await
    {
        Ok(Some(v)) => v,
        Ok(None) => {
            return json_error(
                StatusCode::NOT_FOUND,
                &format!("Bot version {} not found", req.opponent_bot_version_id),
            )
            .into_response();
        }
        Err(e) => return internal_error(e).into_response(),
    };

    // Look up bot names for player display
    let bot_a_name = match state.db.get_bot(version_a.bot_id).await {
        Ok(Some(b)) => b.name,
        _ => format!("Bot v{}", version_a.version),
    };
    let bot_b_name = match state.db.get_bot(version_b.bot_id).await {
        Ok(Some(b)) => b.name,
        _ => format!("Bot v{}", version_b.version),
    };

    // Check rate limits
    let limit_type = if headless {
        RateLimitType::HeadlessChallenges
    } else {
        RateLimitType::LiveChallenges
    };
    if let Err(e) = state.rate_limiter.check_limit(user_id, limit_type) {
        return json_error(StatusCode::TOO_MANY_REQUESTS, &e.to_string()).into_response();
    }

    // For live games, also check concurrent game limit
    if !headless {
        if let Err(e) = state
            .rate_limiter
            .check_limit(user_id, RateLimitType::LiveGames)
        {
            return json_error(StatusCode::TOO_MANY_REQUESTS, &e.to_string()).into_response();
        }
    }

    // Create match record in DB
    let map_name = req.map.clone().unwrap_or_else(|| "random".to_string());
    let m = match state.db.create_match(&format, &map_name).await {
        Ok(m) => m,
        Err(e) => return internal_error(e).into_response(),
    };

    // Add participants
    if let Err(e) = state
        .db
        .add_match_participant(m.id, req.bot_version_id, 0)
        .await
    {
        return internal_error(e).into_response();
    }
    if let Err(e) = state
        .db
        .add_match_participant(m.id, req.opponent_bot_version_id, 1)
        .await
    {
        return internal_error(e).into_response();
    }

    if headless {
        // Queue headless challenge via DB
        if let Err(e) = state
            .db
            .enqueue_game(m.id, req.map.as_deref(), 0, None)
            .await
        {
            tracing::error!("Failed to enqueue headless challenge: {e}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to queue game")
                .into_response();
        }

        return (
            StatusCode::CREATED,
            Json(json!({
                "match_id": m.id,
                "status": "queued",
                "message": "Headless challenge queued."
            })),
        )
            .into_response();
    }

    // Live game: start via GameServer
    if state.game_server.is_running() {
        // Queue it instead of rejecting — priority 0 for ad-hoc challenges
        if let Err(e) = state
            .db
            .enqueue_game(m.id, req.map.as_deref(), 0, None)
            .await
        {
            tracing::error!("Failed to enqueue challenge: {e}");
            return json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to queue game")
                .into_response();
        }

        let queue_depth = state
            .db
            .queue_status()
            .await
            .map(|s| s.pending)
            .unwrap_or(0);

        return (
            StatusCode::CREATED,
            Json(json!({
                "match_id": m.id,
                "status": "queued",
                "message": "A game is already running. Challenge queued.",
                "queue_depth": queue_depth,
            })),
        )
            .into_response();
    }

    // Resolve map
    let world = match resolve_map(&state.maps_dir, &req.map, None) {
        Ok(w) => w,
        Err(e) => {
            return json_error(StatusCode::BAD_REQUEST, &format!("Invalid map: {}", e))
                .into_response();
        }
    };

    let players = vec![
        PlayerEntry {
            name: bot_a_name.clone(),
            code: version_a.code,
        },
        PlayerEntry {
            name: bot_b_name.clone(),
            code: version_b.code,
        },
    ];

    // Build completion callback for Elo, replay, and match finishing
    let version_ids: Vec<i64> = vec![req.bot_version_id, req.opponent_bot_version_id];
    let on_complete =
        build_game_completion_callback(state.db.clone(), m.id, version_ids.clone(), format.clone());

    match state.game_server.start_game_with_callback(
        world,
        players,
        None,
        Some(m.id),
        version_ids,
        false,
        Some(on_complete),
    ) {
        Ok(()) => {
            state.game_server.set_game_map(&map_name);
            (
                StatusCode::CREATED,
                Json(json!({
                    "match_id": m.id,
                    "status": "running",
                    "message": "Game started. Connect to /ws/game for live updates."
                })),
            )
                .into_response()
        }
        Err(e) => json_error(StatusCode::INTERNAL_SERVER_ERROR, &e).into_response(),
    }
}

// ── Queue status handler ─────────────────────────────────────────────

async fn queue_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.queue_status().await {
        Ok(status) => (StatusCode::OK, Json(json!(status))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

// ── Team handlers ──────────────────────────────────────────────────────

async fn list_teams(State(state): State<AppState>, auth: AuthUser) -> impl IntoResponse {
    match state.db.list_teams_by_owner(auth.0.sub).await {
        Ok(teams) => (StatusCode::OK, Json(json!(teams))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn create_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateTeamRequest>,
) -> impl IntoResponse {
    if !crate::auth::has_scope(&auth.0, "teams:write") {
        return json_error(StatusCode::FORBIDDEN, "Insufficient API token scope").into_response();
    }
    if req.name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name is required").into_response();
    }
    match state.db.create_team(auth.0.sub, &req.name).await {
        Ok(team) => (StatusCode::CREATED, Json(json!(team))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn get_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.db.get_team(id).await {
        Ok(Some(team)) => {
            if team.owner_id != auth.0.sub {
                return json_error(StatusCode::FORBIDDEN, "Not your team").into_response();
            }
            (StatusCode::OK, Json(json!(team))).into_response()
        }
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Team not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn update_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<UpdateTeamRequest>,
) -> impl IntoResponse {
    if req.name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name is required").into_response();
    }
    // Check ownership
    match state.db.get_team(id).await {
        Ok(Some(team)) => {
            if team.owner_id != auth.0.sub {
                return json_error(StatusCode::FORBIDDEN, "Not your team").into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Team not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
    }
    match state.db.update_team_name(id, &req.name).await {
        Ok(Some(team)) => (StatusCode::OK, Json(json!(team))).into_response(),
        Ok(None) => json_error(StatusCode::NOT_FOUND, "Team not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn delete_team(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    // Check ownership
    match state.db.get_team(id).await {
        Ok(Some(team)) => {
            if team.owner_id != auth.0.sub {
                return json_error(StatusCode::FORBIDDEN, "Not your team").into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Team not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
    }
    match state.db.delete_team(id).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "Team not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

// ── Team version handlers ──────────────────────────────────────────────

async fn list_team_versions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(team_id): Path<i64>,
) -> impl IntoResponse {
    // Check team exists and ownership
    match state.db.get_team(team_id).await {
        Ok(Some(team)) => {
            if team.owner_id != auth.0.sub {
                return json_error(StatusCode::FORBIDDEN, "Not your team").into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Team not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
    }
    match state.db.list_team_versions(team_id).await {
        Ok(versions) => (StatusCode::OK, Json(json!(versions))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn create_team_version(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(team_id): Path<i64>,
    Json(req): Json<CreateTeamVersionRequest>,
) -> impl IntoResponse {
    // Check team exists and ownership
    match state.db.get_team(team_id).await {
        Ok(Some(team)) => {
            if team.owner_id != auth.0.sub {
                return json_error(StatusCode::FORBIDDEN, "Not your team").into_response();
            }
        }
        Ok(None) => return json_error(StatusCode::NOT_FOUND, "Team not found").into_response(),
        Err(e) => return internal_error(e).into_response(),
    }
    // Verify both bot versions exist
    match state.db.get_bot_version_by_id(req.bot_version_a).await {
        Ok(None) => {
            return json_error(
                StatusCode::BAD_REQUEST,
                &format!("Bot version {} not found", req.bot_version_a),
            )
            .into_response()
        }
        Err(e) => return internal_error(e).into_response(),
        Ok(Some(_)) => {}
    }
    match state.db.get_bot_version_by_id(req.bot_version_b).await {
        Ok(None) => {
            return json_error(
                StatusCode::BAD_REQUEST,
                &format!("Bot version {} not found", req.bot_version_b),
            )
            .into_response()
        }
        Err(e) => return internal_error(e).into_response(),
        Ok(Some(_)) => {}
    }
    match state
        .db
        .create_team_version(team_id, req.bot_version_a, req.bot_version_b)
        .await
    {
        Ok(version) => (StatusCode::CREATED, Json(json!(version))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

// ── API Key handlers ──────────────────────────────────────────────────

async fn list_api_keys(State(state): State<AppState>, auth: AuthUser) -> impl IntoResponse {
    match state.db.list_api_tokens(auth.0.sub).await {
        Ok(tokens) => {
            // Don't expose the hash to the client
            let keys: Vec<serde_json::Value> = tokens
                .iter()
                .map(|t| {
                    json!({
                        "id": t.id,
                        "name": t.name,
                        "scopes": t.scopes,
                        "last_used_at": t.last_used_at,
                        "created_at": t.created_at,
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!(keys))).into_response()
        }
        Err(e) => internal_error(e).into_response(),
    }
}

async fn create_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateApiKeyRequest>,
) -> impl IntoResponse {
    if !crate::auth::has_scope(&auth.0, "api_keys:write") {
        return json_error(StatusCode::FORBIDDEN, "Insufficient API token scope").into_response();
    }
    if req.name.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "name is required").into_response();
    }

    let scopes = req
        .scopes
        .unwrap_or_else(|| "bots:read,matches:read,leaderboard:read".to_string());

    // Generate random token
    let raw_token = format!("sc_{}", hex::encode(generate_random_bytes()));

    // Hash it for storage
    let token_hash = hash_token(&raw_token);

    match state
        .db
        .create_api_token(auth.0.sub, &req.name, &token_hash, &scopes)
        .await
    {
        Ok(token_record) => (
            StatusCode::CREATED,
            Json(json!({
                "id": token_record.id,
                "name": token_record.name,
                "token": raw_token,
                "scopes": token_record.scopes,
                "created_at": token_record.created_at,
            })),
        )
            .into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn delete_api_key(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match state.db.delete_api_token(id, auth.0.sub).await {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "API key not found").into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

fn generate_random_bytes() -> [u8; 32] {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

// ── Game completion callback builder ──────────────────────────────

/// Build a callback closure that handles post-game bookkeeping for live games:
/// saving replays, finishing matches, updating Elo ratings and version stats.
/// This delegates to the shared `run_game_completion` in queue.rs.
pub fn build_game_completion_callback(
    db: Arc<Database>,
    match_id: i64,
    version_ids: Vec<i64>,
    format: String,
) -> Box<dyn FnOnce(GameResult) + Send + 'static> {
    let rt = tokio::runtime::Handle::current();

    Box::new(move |result: GameResult| {
        rt.spawn(async move {
            run_game_completion(&db, match_id, &version_ids, &format, &result).await;
        });
    })
}

// ── Documentation handlers ────────────────────────────────────────────

async fn get_lua_api_docs() -> impl IntoResponse {
    let content = include_str!("../../../docs/lua-api-reference.md");
    (
        StatusCode::OK,
        [("content-type", "text/markdown; charset=utf-8")],
        content,
    )
        .into_response()
}

async fn get_llms_txt() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        crate::llms_txt::LLMS_TXT,
    )
        .into_response()
}

async fn get_llms_full_txt() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        crate::llms_txt::LLMS_FULL_TXT,
    )
        .into_response()
}

// ── Feedback handlers ─────────────────────────────────────────────────

async fn create_feedback(
    State(state): State<AppState>,
    auth: OptionalAuthUser,
    Json(req): Json<CreateFeedbackRequest>,
) -> impl IntoResponse {
    let valid_categories = ["bug", "feature", "general"];
    if !valid_categories.contains(&req.category.as_str()) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "category must be 'bug', 'feature', or 'general'",
        )
        .into_response();
    }
    if req.description.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "description is required").into_response();
    }

    let user_id = auth.0.map(|c| c.sub);
    match state
        .db
        .create_feedback(user_id, &req.category, &req.description)
        .await
    {
        Ok(feedback) => (StatusCode::CREATED, Json(json!(feedback))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}

async fn list_feedback(State(state): State<AppState>, auth: AuthUser) -> impl IntoResponse {
    if auth.0.role != "admin" {
        return json_error(StatusCode::FORBIDDEN, "Admin access required").into_response();
    }
    match state.db.list_feedback().await {
        Ok(feedback) => (StatusCode::OK, Json(json!(feedback))).into_response(),
        Err(e) => internal_error(e).into_response(),
    }
}
