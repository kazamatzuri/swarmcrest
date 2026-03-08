// Game server: manages a running game instance and broadcasts state to WebSocket clients.

use std::panic::AssertUnwindSafe;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::sync::broadcast;

use crate::metrics;
use crate::replay::ReplayRecorder;

use super::config::*;
use super::game::{Game, GameSnapshot, GameSnapshotDelta, PlayerSnapshot, WorldSnapshot};
use super::world::{RandomMapParams, World};

/// Result of a completed game, passed to the on_complete callback.
pub struct GameResult {
    pub match_id: Option<i64>,
    pub winner_player_index: Option<usize>,
    pub player_scores: Vec<PlayerScore>,
    pub replay_data: Vec<u8>,
    pub tick_count: i32,
    /// Bot version IDs that failed to load (e.g. Lua syntax error).
    pub failed_bot_version_ids: Vec<i64>,
}

/// Score data for one player in a completed game.
pub struct PlayerScore {
    pub player_index: usize,
    pub bot_version_id: i64,
    pub score: i32,
    pub creatures_spawned: i32,
    pub creatures_killed: i32,
    pub creatures_lost: i32,
}

/// Metadata about an available map file.
#[derive(Debug, Clone, Serialize)]
pub struct MapInfo {
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub description: String,
}

/// Scan a directory for `*.json` map files and return their metadata, sorted by name.
pub fn list_maps(maps_dir: &Path) -> Vec<MapInfo> {
    let mut maps = Vec::new();

    let entries = match std::fs::read_dir(maps_dir) {
        Ok(e) => e,
        Err(_) => return maps,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        // Parse just enough to get width/height
        #[derive(serde::Deserialize)]
        struct MapMeta {
            width: usize,
            height: usize,
        }
        if let Ok(meta) = serde_json::from_str::<MapMeta>(&contents) {
            maps.push(MapInfo {
                name: stem,
                width: meta.width,
                height: meta.height,
                description: format!("{}x{} map", meta.width, meta.height),
            });
        }
    }

    maps.sort_by(|a, b| a.name.cmp(&b.name));
    maps
}

/// Load a map by name from the given directory. Returns a World or an error message.
pub fn load_map(maps_dir: &Path, name: &str) -> Result<World, String> {
    // Validate map name to prevent path traversal
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!("Invalid map name '{}': only alphanumeric, underscore, and hyphen characters are allowed", name));
    }
    if name.is_empty() {
        return Err("Map name cannot be empty".to_string());
    }

    let path = maps_dir.join(format!("{}.json", name));
    let contents = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read map '{}': {}", name, e))?;
    World::from_json(&contents)
}

/// Messages sent from the game loop to WebSocket clients.
#[derive(Clone, Serialize, Debug)]
#[serde(tag = "type")]
pub enum GameMessage {
    /// Initial world state (tiles, dimensions, koth position).
    #[serde(rename = "world")]
    WorldInit(WorldSnapshot),
    /// Per-tick game state snapshot (full).
    #[serde(rename = "snapshot")]
    Snapshot(GameSnapshot),
    /// Per-tick delta snapshot (only changed creatures).
    #[serde(rename = "snapshot_delta")]
    SnapshotDelta(GameSnapshotDelta),
    /// Game has ended.
    #[serde(rename = "game_end")]
    GameEnd {
        winner: Option<u32>,
        final_scores: Vec<PlayerSnapshot>,
        match_id: Option<i64>,
        player_stats: Vec<PlayerEndStats>,
        game_duration_ticks: u64,
    },
    /// A player failed to load (e.g. Lua syntax error).
    #[serde(rename = "player_load_error")]
    PlayerLoadError { player_name: String, error: String },
}

/// Per-player combat stats included in the GameEnd message.
#[derive(Clone, Serialize, Debug)]
pub struct PlayerEndStats {
    pub player_id: u32,
    pub creatures_spawned: i32,
    pub creatures_killed: i32,
    pub creatures_lost: i32,
}

/// A player entry for starting a game.
pub struct PlayerEntry {
    pub name: String,
    pub code: String,
}

/// Metadata about a currently running game.
#[derive(Debug, Clone, Serialize)]
pub struct ActiveGameInfo {
    pub match_id: Option<i64>,
    pub player_names: Vec<String>,
    pub format: String,
    pub map: String,
    pub start_time: String,
    pub spectator_count: usize,
    pub game_time_seconds: f64,
}

/// Run a game headless (no WebSocket broadcast, no per-tick sleep).
/// Runs synchronously on the calling thread and returns a GameResult.
/// Used by the worker pool for parallel headless game execution.
pub fn run_game_headless(
    world: World,
    players: Vec<PlayerEntry>,
    max_ticks: u64,
    match_id: Option<i64>,
    bot_version_ids: Vec<i64>,
) -> GameResult {
    let format_label = if players.len() == 2 {
        "1v1"
    } else if players.len() > 2 {
        "ffa"
    } else {
        "other"
    }
    .to_string();

    metrics::GAMES_STARTED_TOTAL
        .with_label_values(&[&format_label])
        .inc();
    let game_start_time = std::time::Instant::now();

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let mut game = Game::new(world);
        let mut recorder = ReplayRecorder::new();

        // Add players and spawn initial creatures
        let mut player_ids = Vec::new();
        let mut failed_version_ids: Vec<i64> = Vec::new();
        for (i, entry) in players.iter().enumerate() {
            match game.add_player(&entry.name, &entry.code) {
                Ok(pid) => {
                    player_ids.push(pid);
                }
                Err(e) => {
                    tracing::error!("Failed to add player '{}': {}", entry.name, e);
                    if let Some(&vid) = bot_version_ids.get(i) {
                        failed_version_ids.push(vid);
                    }
                }
            }
        }

        // If too few players loaded successfully, skip the game loop
        let early_exit = player_ids.len() <= 1 && players.len() >= 2;

        // Auto-generate food spawners if the map has none
        game.ensure_food_spawners();
        game.seed_initial_food();

        // Spawn initial creatures
        for &pid in &player_ids {
            for _ in 0..2 {
                let tile = game.world.borrow().find_plain_tile();
                if let Some((tx_pos, ty_pos)) = tile {
                    let cx = World::tile_center(tx_pos);
                    let cy = World::tile_center(ty_pos);
                    game.spawn_creature(pid, cx, cy, CREATURE_SMALL);
                }
            }
        }

        // Record initial world snapshot
        let world_snap = game.world_snapshot();
        let world_msg = GameMessage::WorldInit(world_snap);
        if let Ok(json) = serde_json::to_string(&world_msg) {
            recorder.record_message(&json);
        }

        // Game loop — no sleep, no broadcast
        let mut tick_count: u64 = 0;
        let mut winner: Option<u32> = None;

        if early_exit {
            tracing::info!(
                "Only {} of {} players loaded — skipping game loop",
                player_ids.len(),
                players.len()
            );
            if player_ids.len() == 1 {
                winner = Some(player_ids[0]);
            }
        }

        while !early_exit && tick_count < max_ticks {
            let tick_start = std::time::Instant::now();
            game.tick();
            let tick_elapsed_ms = tick_start.elapsed().as_secs_f64() * 1000.0;
            metrics::GAME_TICK_DURATION_MS.observe(tick_elapsed_ms);
            tick_count += 1;

            // Record snapshot periodically for replay (every 10 ticks)
            if tick_count % 10 == 1 {
                let snapshot = game.snapshot();
                let msg = GameMessage::Snapshot(snapshot);
                if let Ok(json) = serde_json::to_string(&msg) {
                    recorder.record_message(&json);
                }
            }

            // Check win conditions
            if let Some(w) = game.check_score_limit_winner() {
                winner = Some(w);
                break;
            }
            if let Some(w) = game.check_winner() {
                winner = Some(w);
                break;
            }
        }

        // Game ended — determine final result
        let final_snap = game.snapshot();
        let winner = winner.or_else(|| {
            let max_score = final_snap.players.iter().map(|p| p.score).max()?;
            let top: Vec<_> = final_snap
                .players
                .iter()
                .filter(|p| p.score == max_score)
                .collect();
            if top.len() == 1 {
                Some(top[0].id)
            } else {
                None
            }
        });

        // Record final snapshot
        let end_msg = GameMessage::GameEnd {
            winner,
            final_scores: final_snap.players.clone(),
            match_id,
            player_stats: player_ids
                .iter()
                .map(|&pid| {
                    let stats = game.player_stats(pid);
                    PlayerEndStats {
                        player_id: pid,
                        creatures_spawned: stats.creatures_spawned,
                        creatures_killed: stats.creatures_killed,
                        creatures_lost: stats.creatures_lost,
                    }
                })
                .collect(),
            game_duration_ticks: tick_count,
        };
        if let Ok(json) = serde_json::to_string(&end_msg) {
            recorder.record_message(&json);
        }

        let winner_player_index =
            winner.and_then(|winner_id| player_ids.iter().position(|&pid| pid == winner_id));

        let player_scores: Vec<PlayerScore> = final_snap
            .players
            .iter()
            .enumerate()
            .map(|(i, ps)| {
                let pid = player_ids.get(i).copied().unwrap_or(0);
                let stats = game.player_stats(pid);
                PlayerScore {
                    player_index: i,
                    bot_version_id: bot_version_ids.get(i).copied().unwrap_or(0),
                    score: ps.score,
                    creatures_spawned: stats.creatures_spawned,
                    creatures_killed: stats.creatures_killed,
                    creatures_lost: stats.creatures_lost,
                }
            })
            .collect();

        // Record metrics
        let game_elapsed_secs = game_start_time.elapsed().as_secs_f64();
        metrics::GAMES_COMPLETED_TOTAL
            .with_label_values(&[&format_label])
            .inc();
        metrics::GAME_DURATION_SECONDS
            .with_label_values(&[&format_label])
            .observe(game_elapsed_secs);

        GameResult {
            match_id,
            winner_player_index,
            player_scores,
            replay_data: recorder.finish(),
            tick_count: tick_count as i32,
            failed_bot_version_ids: failed_version_ids,
        }
    }));

    match result {
        Ok(game_result) => game_result,
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            tracing::error!("Headless game panicked: {}", msg);
            metrics::GAMES_ERRORED_TOTAL
                .with_label_values(&[&format_label])
                .inc();
            // Return an empty result indicating failure
            GameResult {
                match_id,
                winner_player_index: None,
                player_scores: vec![],
                replay_data: vec![],
                tick_count: 0,
                failed_bot_version_ids: vec![],
            }
        }
    }
}

/// Manages a single game instance, running the game loop on a dedicated thread
/// and broadcasting snapshots to WebSocket subscribers via a broadcast channel.
pub struct GameServer {
    broadcast_tx: broadcast::Sender<String>,
    running: Arc<AtomicBool>,
    /// Cached world JSON so late-joining WS clients get the world state.
    world_json: Arc<Mutex<Option<String>>>,
    /// Metadata about the current game for spectator listing.
    game_meta: Arc<Mutex<Option<GameMeta>>>,
    /// Tick counter updated by game loop thread.
    current_tick: Arc<AtomicI64>,
}

/// Internal metadata stored when a game starts.
#[derive(Debug, Clone)]
struct GameMeta {
    match_id: Option<i64>,
    player_names: Vec<String>,
    format: String,
    map: String,
    start_time: String,
}

impl GameServer {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self {
            broadcast_tx: tx,
            running: Arc::new(AtomicBool::new(false)),
            world_json: Arc::new(Mutex::new(None)),
            game_meta: Arc::new(Mutex::new(None)),
            current_tick: Arc::new(AtomicI64::new(0)),
        }
    }

    /// Subscribe to game messages. Returns a receiver that yields JSON strings.
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.broadcast_tx.subscribe()
    }

    /// Get the cached world JSON for late-joining clients.
    pub fn world_json(&self) -> Option<String> {
        self.world_json.lock().unwrap().clone()
    }

    /// Whether a game is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Stop the currently running game (if any).
    pub fn stop_game(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    /// Get the number of current WebSocket subscribers (spectators).
    pub fn spectator_count(&self) -> usize {
        // receiver_count() returns the number of active receivers on the broadcast channel.
        // Subtract 1 because the game loop itself is not a spectator, but actually
        // the game loop uses tx.send(), not subscribe(), so receiver_count is accurate.
        self.broadcast_tx.receiver_count()
    }

    /// Get information about the currently active game, if any.
    pub fn active_game_info(&self) -> Option<ActiveGameInfo> {
        if !self.is_running() {
            return None;
        }
        let meta = self.game_meta.lock().unwrap().clone()?;
        let tick = self.current_tick.load(Ordering::Relaxed);
        Some(ActiveGameInfo {
            match_id: meta.match_id,
            player_names: meta.player_names,
            format: meta.format,
            map: meta.map,
            start_time: meta.start_time,
            spectator_count: self.spectator_count(),
            game_time_seconds: tick as f64 * 0.1, // 100ms per tick
        })
    }

    /// Start a game with the given world and players.
    /// The game loop runs on a dedicated OS thread (Game is !Send due to Rc<RefCell<>>).
    /// Each tick broadcasts a GameSnapshot as JSON to all subscribers.
    /// The game runs for `max_ticks` ticks (default 6000 = 10 minutes at 100ms/tick).
    pub fn start_game(
        &self,
        world: World,
        players: Vec<PlayerEntry>,
        max_ticks: Option<u64>,
    ) -> Result<(), String> {
        self.start_game_with_callback(world, players, max_ticks, None, vec![], false, None)
    }

    /// Start a game with a completion callback for Elo updates, replay saving, etc.
    ///
    /// - `match_id`: optional DB match ID to include in the GameResult
    /// - `bot_version_ids`: one per player, same order as `players` vec
    /// - `headless`: if true, skip the 100ms per-tick sleep (fast mode)
    /// - `on_complete`: called on the game thread when the game finishes
    pub fn start_game_with_callback(
        &self,
        world: World,
        players: Vec<PlayerEntry>,
        max_ticks: Option<u64>,
        match_id: Option<i64>,
        bot_version_ids: Vec<i64>,
        headless: bool,
        on_complete: Option<Box<dyn FnOnce(GameResult) + Send + 'static>>,
    ) -> Result<(), String> {
        if self.is_running() {
            return Err("A game is already running".into());
        }

        let tx = self.broadcast_tx.clone();
        let running = self.running.clone();
        let world_json = self.world_json.clone();
        let game_meta = self.game_meta.clone();
        let current_tick = self.current_tick.clone();
        let max_ticks = max_ticks.unwrap_or(6000);

        // Store game metadata for active game listing
        let player_names: Vec<String> = players.iter().map(|p| p.name.clone()).collect();
        let format = if players.len() == 2 {
            "1v1".to_string()
        } else {
            "ffa".to_string()
        };
        *game_meta.lock().unwrap() = Some(GameMeta {
            match_id,
            player_names,
            format,
            map: "unknown".to_string(), // Will be overridden by callers via set_game_map
            start_time: chrono::Utc::now().to_rfc3339(),
        });
        current_tick.store(0, Ordering::Relaxed);

        running.store(true, Ordering::Relaxed);

        // Determine format label for metrics
        let format_label = if players.len() == 2 {
            "1v1"
        } else if players.len() > 2 {
            "ffa"
        } else {
            "other"
        }
        .to_string();

        metrics::ACTIVE_GAMES.set(1);
        metrics::GAMES_STARTED_TOTAL
            .with_label_values(&[&format_label])
            .inc();
        let game_start_time = std::time::Instant::now();

        std::thread::spawn(move || {
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
                let mut game = Game::new(world);
                let mut recorder = ReplayRecorder::new();

                // Add players and spawn initial creatures
                let mut player_ids = Vec::new();
                let mut failed_version_ids: Vec<i64> = Vec::new();
                for (i, entry) in players.iter().enumerate() {
                    match game.add_player(&entry.name, &entry.code) {
                        Ok(pid) => {
                            player_ids.push(pid);
                        }
                        Err(e) => {
                            tracing::error!("Failed to add player '{}': {}", entry.name, e);
                            if let Some(&vid) = bot_version_ids.get(i) {
                                failed_version_ids.push(vid);
                            }
                            let err_msg = GameMessage::PlayerLoadError {
                                player_name: entry.name.clone(),
                                error: e.to_string(),
                            };
                            if let Ok(json) = serde_json::to_string(&err_msg) {
                                let _ = tx.send(json);
                            }
                        }
                    }
                }

                // If too few players loaded successfully, skip the game loop
                let early_exit = player_ids.len() <= 1 && players.len() >= 2;

                // Auto-generate food spawners if the map has none
                game.ensure_food_spawners();

                // Place initial food from spawners
                game.seed_initial_food();

                // Spawn initial creatures for each player on random walkable tiles
                for &pid in &player_ids {
                    let initial_creatures = 2;
                    for _ in 0..initial_creatures {
                        let tile = game.world.borrow().find_plain_tile();
                        if let Some((tx_pos, ty_pos)) = tile {
                            let cx = World::tile_center(tx_pos);
                            let cy = World::tile_center(ty_pos);
                            game.spawn_creature(pid, cx, cy, CREATURE_SMALL);
                        }
                    }
                }

                // Send initial world snapshot and cache it for late joiners
                let world_snap = game.world_snapshot();
                let world_msg = GameMessage::WorldInit(world_snap);
                if let Ok(json) = serde_json::to_string(&world_msg) {
                    *world_json.lock().unwrap() = Some(json.clone());
                    let _ = tx.send(json.clone());
                    recorder.record_message(&json);
                }

                // Game loop with delta compression
                let mut tick_count: u64 = 0;
                let mut winner: Option<u32> = None;
                let mut prev_snapshot: Option<GameSnapshot> = None;
                const FULL_SNAPSHOT_INTERVAL: u64 = 10;

                // Skip game loop if not enough players loaded
                if early_exit {
                    tracing::info!(
                        "Only {} of {} players loaded — skipping game loop",
                        player_ids.len(),
                        players.len()
                    );
                    // If exactly 1 player loaded, they win by default
                    if player_ids.len() == 1 {
                        winner = Some(player_ids[0]);
                    }
                }

                while !early_exit && running.load(Ordering::Relaxed) && tick_count < max_ticks {
                    let tick_start = std::time::Instant::now();
                    game.tick();
                    let tick_elapsed_ms = tick_start.elapsed().as_secs_f64() * 1000.0;
                    metrics::GAME_TICK_DURATION_MS.observe(tick_elapsed_ms);
                    tick_count += 1;
                    current_tick.store(tick_count as i64, Ordering::Relaxed);

                    let snapshot = game.snapshot();

                    // Send full snapshot every N ticks, delta in between
                    let send_full =
                        tick_count % FULL_SNAPSHOT_INTERVAL == 1 || prev_snapshot.is_none();

                    if send_full {
                        let msg = GameMessage::Snapshot(snapshot.clone());
                        if let Ok(json) = serde_json::to_string(&msg) {
                            let _ = tx.send(json.clone());
                            recorder.record_message(&json);
                        }
                    } else if let Some(ref prev) = prev_snapshot {
                        let delta = Game::compute_delta(&snapshot, prev);
                        // Only send delta if it's smaller than full (fallback to full)
                        let delta_msg = GameMessage::SnapshotDelta(delta);
                        if let Ok(delta_json) = serde_json::to_string(&delta_msg) {
                            let full_msg = GameMessage::Snapshot(snapshot.clone());
                            if let Ok(full_json) = serde_json::to_string(&full_msg) {
                                if delta_json.len() < full_json.len() {
                                    let _ = tx.send(delta_json.clone());
                                    recorder.record_message(&delta_json);
                                } else {
                                    let _ = tx.send(full_json.clone());
                                    recorder.record_message(&full_json);
                                }
                            }
                        }
                    }

                    prev_snapshot = Some(snapshot);

                    // Check win conditions
                    if let Some(w) = game.check_score_limit_winner() {
                        tracing::info!(player_id = w, "Player won — reached score limit");
                        winner = Some(w);
                        break;
                    }
                    if let Some(w) = game.check_winner() {
                        tracing::info!(player_id = w, "Player won — last one standing");
                        winner = Some(w);
                        break;
                    }

                    if !headless {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }

                // Game ended -- send final scores
                let final_snap = game.snapshot();
                // Time-limit tiebreak: highest score wins, ties are draws
                let winner = winner.or_else(|| {
                    let max_score = final_snap.players.iter().map(|p| p.score).max()?;
                    let top: Vec<_> = final_snap
                        .players
                        .iter()
                        .filter(|p| p.score == max_score)
                        .collect();
                    if top.len() == 1 {
                        Some(top[0].id)
                    } else {
                        tracing::info!(
                            "Time limit reached — draw ({} players tied at {max_score})",
                            top.len()
                        );
                        None
                    }
                });

                let end_player_stats: Vec<PlayerEndStats> = player_ids
                    .iter()
                    .map(|&pid| {
                        let stats = game.player_stats(pid);
                        PlayerEndStats {
                            player_id: pid,
                            creatures_spawned: stats.creatures_spawned,
                            creatures_killed: stats.creatures_killed,
                            creatures_lost: stats.creatures_lost,
                        }
                    })
                    .collect();

                let end_msg = GameMessage::GameEnd {
                    winner,
                    final_scores: final_snap.players.clone(),
                    match_id,
                    player_stats: end_player_stats,
                    game_duration_ticks: tick_count,
                };
                if let Ok(json) = serde_json::to_string(&end_msg) {
                    let _ = tx.send(json.clone());
                    recorder.record_message(&json);
                }

                // Build GameResult and invoke callback
                if let Some(callback) = on_complete {
                    // Determine winner player index (index into players vec)
                    let winner_player_index = winner
                        .and_then(|winner_id| player_ids.iter().position(|&pid| pid == winner_id));

                    // Build player scores from the final snapshot
                    let player_scores: Vec<PlayerScore> = final_snap
                        .players
                        .iter()
                        .enumerate()
                        .map(|(i, ps)| {
                            let pid = player_ids.get(i).copied().unwrap_or(0);
                            let stats = game.player_stats(pid);
                            PlayerScore {
                                player_index: i,
                                bot_version_id: bot_version_ids.get(i).copied().unwrap_or(0),
                                score: ps.score,
                                creatures_spawned: stats.creatures_spawned,
                                creatures_killed: stats.creatures_killed,
                                creatures_lost: stats.creatures_lost,
                            }
                        })
                        .collect();

                    let game_result = GameResult {
                        match_id,
                        winner_player_index,
                        player_scores,
                        replay_data: recorder.finish(),
                        tick_count: tick_count as i32,
                        failed_bot_version_ids: failed_version_ids.clone(),
                    };

                    callback(game_result);
                }

                // Record game completion metrics
                let game_elapsed_secs = game_start_time.elapsed().as_secs_f64();
                metrics::GAMES_COMPLETED_TOTAL
                    .with_label_values(&[&format_label])
                    .inc();
                metrics::GAME_DURATION_SECONDS
                    .with_label_values(&[&format_label])
                    .observe(game_elapsed_secs);
            }));

            if let Err(panic_info) = result {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                tracing::error!("Game thread panicked: {}", msg);
                metrics::GAMES_ERRORED_TOTAL
                    .with_label_values(&[&format_label])
                    .inc();
            }

            metrics::ACTIVE_GAMES.set(0);
            *world_json.lock().unwrap() = None;
            *game_meta.lock().unwrap() = None;
            current_tick.store(0, Ordering::Relaxed);
            running.store(false, Ordering::Relaxed);
        });

        Ok(())
    }

    /// Set the map name on the current game metadata.
    /// Called after start_game_with_callback by the API layer which knows the map name.
    pub fn set_game_map(&self, map: &str) {
        if let Some(ref mut meta) = *self.game_meta.lock().unwrap() {
            meta.map = map.to_string();
        }
    }

    /// Create a default world using random map generation.
    pub fn default_world() -> World {
        World::generate_random(RandomMapParams::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_server_new() {
        let server = GameServer::new();
        assert!(!server.is_running());
    }

    #[test]
    fn test_default_world() {
        let world = GameServer::default_world();
        assert_eq!(world.width, 30);
        assert_eq!(world.height, 30);
        // Border should always be solid
        assert!(!world.is_walkable(0, 0));
        assert!(!world.is_walkable(29, 29));
        // Should have some walkable tiles
        assert!(world.find_plain_tile().is_some());
        // Should have food spawners
        assert!(!world.food_spawners.is_empty());
    }

    #[test]
    fn test_game_message_serialization() {
        let snap = GameSnapshot {
            game_time: 100,
            creatures: vec![],
            players: vec![PlayerSnapshot {
                id: 1,
                name: "test".to_string(),
                score: 42,
                color: 0,
                num_creatures: 3,
                output: vec![],
            }],
            king_player_id: Some(1),
            events: vec![],
        };
        let msg = GameMessage::Snapshot(snap);
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"snapshot\""));
        assert!(json.contains("\"game_time\":100"));

        let end_msg = GameMessage::GameEnd {
            winner: Some(1),
            final_scores: vec![],
            match_id: None,
            player_stats: vec![],
            game_duration_ticks: 100,
        };
        let json = serde_json::to_string(&end_msg).unwrap();
        assert!(json.contains("\"type\":\"game_end\""));
    }

    #[test]
    fn test_load_map_rejects_path_traversal() {
        let dir = std::path::Path::new("/tmp");
        assert!(load_map(dir, "../etc/passwd").is_err());
        assert!(load_map(dir, "../../secret").is_err());
        assert!(load_map(dir, "maps/../../etc").is_err());
        assert!(load_map(dir, "").is_err());
    }

    #[test]
    fn test_load_map_accepts_valid_names() {
        // These should not error on validation (may error on file not found, which is fine)
        let dir = std::path::Path::new("/tmp");
        let result = load_map(dir, "valid-map");
        assert!(result.is_err()); // file not found, but not validation error
        match result {
            Err(e) => assert!(!e.contains("Invalid map name")),
            Ok(_) => panic!("expected error"),
        }

        let result = load_map(dir, "map_123");
        assert!(result.is_err());
        match result {
            Err(e) => assert!(!e.contains("Invalid map name")),
            Ok(_) => panic!("expected error"),
        }
    }

    #[test]
    fn test_start_game_while_running() {
        let server = GameServer::new();
        let world = GameServer::default_world();
        // Start a game
        let result = server.start_game(world, vec![], Some(5));
        assert!(result.is_ok());

        // Brief sleep to let thread start
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Try to start another while first is running
        let world2 = GameServer::default_world();
        let result2 = server.start_game(world2, vec![], Some(5));
        assert!(result2.is_err());

        server.stop_game();
        // Wait for thread to finish
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}
