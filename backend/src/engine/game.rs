use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;

use serde::Serialize;

use super::config::*;
use super::creature::Creature;
use super::lua_api::{self, LuaGameState};
use super::player::Player;
use super::spatial::SpatialGrid;
use super::world::World;

/// Map a creature type constant to a string label for metrics.
fn creature_type_label(creature_type: u8) -> &'static str {
    match creature_type {
        CREATURE_SMALL => "small",
        CREATURE_BIG => "big",
        CREATURE_FLYER => "flyer",
        _ => "unknown",
    }
}

/// Per-player game statistics tracked during gameplay.
#[derive(Debug, Default, Clone)]
pub struct PlayerStats {
    pub creatures_spawned: i32,
    pub creatures_killed: i32,
    pub creatures_lost: i32,
}

/// Events that get passed to player_think Lua function.
#[derive(Clone, Debug)]
pub enum GameEvent {
    CreatureSpawned { id: u32, parent: i32 },
    CreatureKilled { id: u32, killer: i32 },
    CreatureAttacked { id: u32, attacker: u32 },
    PlayerCreated { player_id: u32 },
}

/// Events broadcast to WebSocket clients for the event ticker.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind")]
pub enum BroadcastEvent {
    Spawn {
        creature_id: u32,
        player_id: u32,
        player_name: String,
        creature_type: u8,
    },
    Kill {
        creature_id: u32,
        player_id: u32,
        player_name: String,
        killer_player_id: Option<u32>,
        killer_player_name: Option<String>,
        starvation: bool,
    },
    PlayerJoined {
        player_id: u32,
        player_name: String,
    },
}

/// Snapshot of a creature for rendering / API consumers.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct CreatureSnapshot {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub creature_type: u8,
    pub health: i32,
    pub max_health: i32,
    pub food: i32,
    pub state: u8,
    pub player_id: u32,
    pub message: String,
    pub target_id: Option<u32>,
}

/// Snapshot of a player for rendering / API consumers.
#[derive(Clone, Debug, Serialize)]
pub struct PlayerSnapshot {
    pub id: u32,
    pub name: String,
    pub score: i32,
    pub color: u8,
    pub num_creatures: i32,
    pub output: Vec<String>,
}

/// Snapshot of a tile for rendering / API consumers.
#[derive(Clone, Debug, Serialize)]
pub struct TileSnapshot {
    pub x: usize,
    pub y: usize,
    pub food: i32,
    pub tile_type: u8,
    pub gfx: u8,
}

/// Full world snapshot sent on initial connection.
#[derive(Clone, Debug, Serialize)]
pub struct WorldSnapshot {
    pub width: usize,
    pub height: usize,
    pub koth_x: usize,
    pub koth_y: usize,
    pub tiles: Vec<TileSnapshot>,
}

/// Snapshot of the full game state for one tick.
#[derive(Clone, Debug, Serialize)]
pub struct GameSnapshot {
    pub game_time: i64,
    pub creatures: Vec<CreatureSnapshot>,
    pub players: Vec<PlayerSnapshot>,
    pub king_player_id: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<BroadcastEvent>,
}

/// Delta snapshot: only creatures that changed since the last full snapshot.
/// The frontend merges this with the last full snapshot.
#[derive(Clone, Debug, Serialize)]
pub struct GameSnapshotDelta {
    pub game_time: i64,
    /// Creatures that were added or changed since last snapshot.
    pub changed: Vec<CreatureSnapshot>,
    /// IDs of creatures that were removed since last snapshot.
    pub removed: Vec<u32>,
    /// Player data (always sent in full since it's small).
    pub players: Vec<PlayerSnapshot>,
    pub king_player_id: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<BroadcastEvent>,
}

/// Tick timing data for observability and budget monitoring.
#[derive(Clone, Debug, Default)]
pub struct TickTimings {
    /// Total tick duration in microseconds.
    pub total_us: u64,
    /// Time spent in player_think (Lua) in microseconds.
    pub think_us: u64,
    /// Time spent processing creatures in microseconds.
    pub creatures_us: u64,
    /// Per-player think time in microseconds (player_id -> us).
    pub per_player_think_us: Vec<(u32, u64)>,
}

/// Top-level game state and tick loop.
pub struct Game {
    pub world: Rc<RefCell<World>>,
    pub creatures: Rc<RefCell<HashMap<u32, Creature>>>,
    pub players: HashMap<u32, Player>,
    pub game_time: i64,
    pub next_creature_id: u32,
    pub next_player_id: u32,
    pub king_player_id: Option<u32>,
    pub king_time: i32,
    pub tick_delta: i32,
    pub score_limit: Option<i32>,
    pub player_scores: Rc<RefCell<HashMap<u32, i32>>>,
    pub player_names: Rc<RefCell<HashMap<u32, String>>>,
    /// Pending events per player (player_id -> events)
    pending_events: HashMap<u32, Vec<GameEvent>>,
    /// Events to broadcast to WebSocket clients (drained each snapshot).
    broadcast_events: Vec<BroadcastEvent>,
    /// Per-player statistics (spawns, kills, losses)
    player_stats: HashMap<u32, PlayerStats>,
    /// Spatial index for fast creature proximity queries. Rebuilt each tick.
    spatial_grid: Rc<RefCell<SpatialGrid>>,
    /// Timing data from the last tick.
    pub last_tick_timings: TickTimings,
}

impl Game {
    pub fn new(world: World) -> Self {
        let grid = SpatialGrid::new(world.width, world.height);
        Game {
            world: Rc::new(RefCell::new(world)),
            creatures: Rc::new(RefCell::new(HashMap::new())),
            players: HashMap::new(),
            game_time: 0,
            next_creature_id: 1,
            next_player_id: 1,
            king_player_id: None,
            king_time: 0,
            tick_delta: 100,
            score_limit: Some(500),
            player_scores: Rc::new(RefCell::new(HashMap::new())),
            player_names: Rc::new(RefCell::new(HashMap::new())),
            pending_events: HashMap::new(),
            broadcast_events: Vec::new(),
            player_stats: HashMap::new(),
            spatial_grid: Rc::new(RefCell::new(grid)),
            last_tick_timings: TickTimings::default(),
        }
    }

    /// Add a player with the given bot code.
    /// Returns the player ID on success.
    pub fn add_player(&mut self, name: &str, code: &str) -> Result<u32, String> {
        let player_id = self.next_player_id;
        self.next_player_id += 1;

        let player = Player::new(player_id, name)?;

        // Set game state so top-level bot code can call API functions
        // (e.g. world_size(), get_koth_pos() during script initialization)
        let print_output = Rc::new(RefCell::new(Vec::new()));
        let gs = Rc::new(RefCell::new(lua_api::LuaGameState {
            world: self.world.clone(),
            creatures: self.creatures.clone(),
            game_time: self.game_time,
            player_id,
            player_scores: self.player_scores.clone(),
            player_names: self.player_names.clone(),
            king_player_id: self.king_player_id,
            print_output: print_output.clone(),
            spatial_grid: Some(self.spatial_grid.clone()),
        }));
        lua_api::set_game_state(&player.lua, gs);

        let load_result = player.load_code(code);

        lua_api::clear_game_state(&player.lua);

        // Collect any print output from loading
        {
            let output = print_output.borrow();
            // Can't mutate player yet since we need to insert it first
            // We'll store output after insert
            drop(output);
        }

        load_result?;

        self.players.insert(player_id, player);

        // Collect load-time print output
        {
            let output = print_output.borrow();
            if let Some(p) = self.players.get_mut(&player_id) {
                p.output.extend(output.iter().cloned());
            }
        }

        self.player_scores.borrow_mut().insert(player_id, 0);
        self.player_names
            .borrow_mut()
            .insert(player_id, name.to_string());
        self.pending_events.insert(player_id, Vec::new());

        // Queue PLAYER_CREATED event
        self.pending_events
            .entry(player_id)
            .or_default()
            .push(GameEvent::PlayerCreated { player_id });

        // Broadcast player joined event
        self.broadcast_events.push(BroadcastEvent::PlayerJoined {
            player_id,
            player_name: name.to_string(),
        });

        Ok(player_id)
    }

    /// Remove a player and all their creatures.
    pub fn remove_player(&mut self, player_id: u32) {
        self.players.remove(&player_id);
        self.player_scores.borrow_mut().remove(&player_id);
        self.player_names.borrow_mut().remove(&player_id);
        self.pending_events.remove(&player_id);
        self.player_stats.remove(&player_id);

        // Kill all creatures belonging to this player
        let to_kill: Vec<u32> = self
            .creatures
            .borrow()
            .iter()
            .filter(|(_, c)| c.player_id == player_id)
            .map(|(id, _)| *id)
            .collect();
        for id in to_kill {
            self.creatures.borrow_mut().remove(&id);
        }
    }

    /// Get the stats for a player. Returns default (all zeros) if not found.
    pub fn player_stats(&self, player_id: u32) -> PlayerStats {
        self.player_stats
            .get(&player_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Spawn a creature for a player at the given pixel position.
    /// Returns the creature ID or None if spawn fails.
    pub fn spawn_creature(
        &mut self,
        player_id: u32,
        x: i32,
        y: i32,
        creature_type: u8,
    ) -> Option<u32> {
        if !self.players.contains_key(&player_id) {
            return None;
        }

        let id = self.next_creature_id;
        self.next_creature_id += 1;

        let creature = Creature::new(id, x, y, creature_type, player_id);
        self.creatures.borrow_mut().insert(id, creature);

        // Update player creature count
        if let Some(player) = self.players.get_mut(&player_id) {
            player.num_creatures += 1;
        }

        // Track spawn stat
        self.player_stats
            .entry(player_id)
            .or_default()
            .creatures_spawned += 1;

        // Metrics
        crate::metrics::CREATURES_SPAWNED_TOTAL
            .with_label_values(&[creature_type_label(creature_type)])
            .inc();

        // Queue spawn event for the owning player
        self.pending_events
            .entry(player_id)
            .or_default()
            .push(GameEvent::CreatureSpawned { id, parent: -1 });

        // Broadcast spawn event
        let player_name = self
            .player_names
            .borrow()
            .get(&player_id)
            .cloned()
            .unwrap_or_default();
        self.broadcast_events.push(BroadcastEvent::Spawn {
            creature_id: id,
            player_id,
            player_name,
            creature_type,
        });

        Some(id)
    }

    /// Spawn a creature as an offspring of another creature.
    fn spawn_offspring(
        &mut self,
        parent_id: u32,
        player_id: u32,
        x: i32,
        y: i32,
        creature_type: u8,
    ) -> Option<u32> {
        let id = self.next_creature_id;
        self.next_creature_id += 1;

        let creature = Creature::new(id, x, y, creature_type, player_id);
        self.creatures.borrow_mut().insert(id, creature);

        if let Some(player) = self.players.get_mut(&player_id) {
            player.num_creatures += 1;
        }

        // Track spawn stat
        self.player_stats
            .entry(player_id)
            .or_default()
            .creatures_spawned += 1;

        // Score bonus for spawning
        self.change_player_score(player_id, 10);

        // Metrics
        crate::metrics::CREATURES_SPAWNED_TOTAL
            .with_label_values(&[creature_type_label(creature_type)])
            .inc();

        self.pending_events
            .entry(player_id)
            .or_default()
            .push(GameEvent::CreatureSpawned {
                id,
                parent: parent_id as i32,
            });

        // Broadcast spawn event
        let player_name = self
            .player_names
            .borrow()
            .get(&player_id)
            .cloned()
            .unwrap_or_default();
        self.broadcast_events.push(BroadcastEvent::Spawn {
            creature_id: id,
            player_id,
            player_name,
            creature_type,
        });

        Some(id)
    }

    /// Change a player's score by delta points.
    fn change_player_score(&mut self, player_id: u32, delta: i32) {
        self.player_scores
            .borrow_mut()
            .entry(player_id)
            .and_modify(|s| *s += delta)
            .or_insert(delta);
        if let Some(player) = self.players.get_mut(&player_id) {
            player.score += delta;
        }
    }

    /// Kill a creature. Queues kill event.
    pub fn kill_creature(&mut self, creature_id: u32, killer_id: Option<u32>) {
        let creature = self
            .creatures
            .borrow()
            .get(&creature_id)
            .map(|c| (c.player_id, c.id, c.creature_type));
        if let Some((player_id, _, ctype)) = creature {
            // Metrics
            crate::metrics::CREATURES_KILLED_TOTAL
                .with_label_values(&[creature_type_label(ctype)])
                .inc();
            // Track creatures_lost for the owner
            self.player_stats
                .entry(player_id)
                .or_default()
                .creatures_lost += 1;

            // Track creatures_killed for the killer (if it's a different player)
            if let Some(kid) = killer_id {
                let killer_player_id = self.creatures.borrow().get(&kid).map(|c| c.player_id);
                if let Some(kpid) = killer_player_id {
                    if kpid != player_id {
                        self.player_stats.entry(kpid).or_default().creatures_killed += 1;
                    }
                }
            }

            // Score changes based on kill type
            if let Some(kid) = killer_id {
                if kid == creature_id {
                    // Suicide
                    self.change_player_score(player_id, -40);
                } else {
                    let killer_info = self
                        .creatures
                        .borrow()
                        .get(&kid)
                        .map(|c| (c.player_id, c.creature_type));
                    if let Some((killer_player_id, killer_type)) = killer_info {
                        match (ctype, killer_type) {
                            (CREATURE_SMALL, CREATURE_BIG) => {
                                self.change_player_score(player_id, -3);
                                self.change_player_score(killer_player_id, 10);
                            }
                            (CREATURE_BIG, CREATURE_BIG) => {
                                self.change_player_score(player_id, -8);
                                self.change_player_score(killer_player_id, 15);
                            }
                            (CREATURE_FLYER, CREATURE_SMALL) | (CREATURE_FLYER, CREATURE_BIG) => {
                                self.change_player_score(player_id, -4);
                                self.change_player_score(killer_player_id, 12);
                            }
                            _ => {} // Should not happen based on combat rules
                        }
                    }
                }
            } else {
                // Starvation death
                self.change_player_score(player_id, -3);
            }

            self.pending_events
                .entry(player_id)
                .or_default()
                .push(GameEvent::CreatureKilled {
                    id: creature_id,
                    killer: killer_id.map(|k| k as i32).unwrap_or(-1),
                });

            // Broadcast kill event
            {
                let names = self.player_names.borrow();
                let player_name = names.get(&player_id).cloned().unwrap_or_default();
                let starvation = killer_id.is_none();
                let (killer_player_id, killer_player_name) = if let Some(kid) = killer_id {
                    if kid == creature_id {
                        // Suicide — killer is self
                        (Some(player_id), Some(player_name.clone()))
                    } else {
                        let kpid = self.creatures.borrow().get(&kid).map(|c| c.player_id);
                        let kname = kpid.and_then(|id| names.get(&id).cloned());
                        (kpid, kname)
                    }
                } else {
                    (None, None)
                };
                self.broadcast_events.push(BroadcastEvent::Kill {
                    creature_id,
                    player_id,
                    player_name,
                    killer_player_id,
                    killer_player_name,
                    starvation,
                });
            }

            // Drop food on tile
            let creatures = self.creatures.borrow();
            if let Some(c) = creatures.get(&creature_id) {
                let tx = c.tile_x();
                let ty = c.tile_y();
                let mut food = c.food;
                // Suicide drops only 1/3 of food
                if killer_id == Some(creature_id) {
                    food /= 3;
                }
                drop(creatures);
                if food > 0 {
                    self.world.borrow_mut().add_food(tx, ty, food);
                }
            } else {
                drop(creatures);
            }

            self.creatures.borrow_mut().remove(&creature_id);

            if let Some(player) = self.players.get_mut(&player_id) {
                player.num_creatures -= 1;
            }
        }
    }

    /// Run one game tick.
    pub fn tick(&mut self) {
        let tick_start = Instant::now();
        let delta = self.tick_delta;

        // 0. Rebuild spatial index from current creature positions
        self.rebuild_spatial_index();

        // 1. Run each player's think (Lua execution)
        let think_start = Instant::now();
        self.process_player_think();
        let think_us = think_start.elapsed().as_micros() as u64;

        // 2. Process all creatures (movement, combat, aging, etc.)
        let creatures_start = Instant::now();
        self.process_creatures(delta);
        let creatures_us = creatures_start.elapsed().as_micros() as u64;

        // 3. King of the Hill scoring
        self.process_koth();

        // 4. Food spawning
        self.process_food_spawners();

        // 5. Advance game time
        self.game_time += delta as i64;

        // 6. Record tick timings
        let total_us = tick_start.elapsed().as_micros() as u64;
        self.last_tick_timings = TickTimings {
            total_us,
            think_us,
            creatures_us,
            per_player_think_us: Vec::new(), // populated during process_player_think
        };

        // Warn if tick took too long (>50ms for a 100ms tick budget)
        if total_us > 50_000 {
            tracing::warn!(
                game_time = self.game_time,
                total_us,
                think_us,
                creatures_us,
                creature_count = self.creatures.borrow().len(),
                "Tick exceeded budget (>50ms)"
            );
        }
    }

    /// Rebuild the spatial index from current creature positions.
    fn rebuild_spatial_index(&self) {
        let mut grid = self.spatial_grid.borrow_mut();
        grid.clear();
        let creatures = self.creatures.borrow();
        for (_, c) in creatures.iter() {
            grid.insert(c.id, c.x, c.y, c.player_id);
        }
    }

    /// Spawn food from map food spawners. Each spawner places food at a random tile
    /// within its radius every `interval` ticks.
    fn process_food_spawners(&mut self) {
        use rand::Rng;
        let mut world = self.world.borrow_mut();
        let game_time = self.game_time;
        let spawners = world.food_spawners.clone();

        for spawner in &spawners {
            // interval is in milliseconds (matching original Lua game_time units)
            let interval_ms = spawner.interval as i64;
            if interval_ms <= 0 {
                continue;
            }
            // Only spawn on the tick boundary matching this interval
            if game_time % interval_ms != 0 {
                continue;
            }
            // Place food at a random tile: offset by 0..radius (positive only, like original)
            let mut rng = rand::thread_rng();
            let r = spawner.radius.max(1) as i32;
            let tx = spawner.x as i32 + rng.gen_range(0..=r);
            let ty = spawner.y as i32 + rng.gen_range(0..=r);
            if tx >= 0 && ty >= 0 {
                let tx = tx as usize;
                let ty = ty as usize;
                if world.is_walkable(tx, ty) {
                    world.add_food(tx, ty, spawner.amount);
                }
            }
        }
    }

    /// Ensure the map has enough food spawners. If fewer than 10, add random ones on
    /// walkable tiles (like the original game's world_find_digged() spawners).
    pub fn ensure_food_spawners(&mut self) {
        use super::world::FoodSpawner;
        use rand::Rng;

        let mut world = self.world.borrow_mut();
        let existing = world.food_spawners.len();
        let target = 15;
        if existing >= target {
            return;
        }

        let mut rng = rand::thread_rng();
        let walkable: Vec<(usize, usize)> = (0..world.width)
            .flat_map(|x| (0..world.height).map(move |y| (x, y)))
            .filter(|&(x, y)| world.is_walkable(x, y))
            .collect();

        if walkable.is_empty() {
            return;
        }

        let to_add = (target - existing).min(walkable.len());
        for _ in 0..to_add {
            let idx = rng.gen_range(0..walkable.len());
            let (x, y) = walkable[idx];
            world.food_spawners.push(FoodSpawner {
                x,
                y,
                radius: rng.gen_range(2..=5),
                amount: 500,
                interval: 2000,
            });
        }
    }

    /// Place initial food from spawners so maps start with food already on them.
    /// Mirrors the original game behavior: each spawner gets a big initial food dump.
    pub fn seed_initial_food(&mut self) {
        let mut world = self.world.borrow_mut();

        for spawner in &world.food_spawners.clone() {
            // Place a large initial food pile at the spawner's center tile
            // (original game does world_add_food(spawner.x, spawner.y, 10000))
            if world.is_walkable(spawner.x, spawner.y) {
                world.add_food(spawner.x, spawner.y, 9000);
            }
        }
    }

    /// Run each player's Lua think function.
    fn process_player_think(&mut self) {
        let player_ids: Vec<u32> = self.players.keys().copied().collect();

        for pid in player_ids {
            // Take pending events for this player
            let events = self
                .pending_events
                .get_mut(&pid)
                .map(std::mem::take)
                .unwrap_or_default();

            // Build the Lua events table
            let player = match self.players.get(&pid) {
                Some(p) => p,
                None => continue,
            };

            let print_output = Rc::new(RefCell::new(Vec::new()));

            let gs = Rc::new(RefCell::new(LuaGameState {
                world: self.world.clone(),
                creatures: self.creatures.clone(),
                game_time: self.game_time,
                player_id: pid,
                player_scores: self.player_scores.clone(),
                player_names: self.player_names.clone(),
                king_player_id: self.king_player_id,
                print_output: print_output.clone(),
                spatial_grid: Some(self.spatial_grid.clone()),
            }));

            lua_api::set_game_state(&player.lua, gs);

            // Set instruction count limit to prevent infinite loops
            player.lua.set_hook(
                mlua::HookTriggers::new().every_nth_instruction(LUA_MAX_INSTRUCTIONS),
                |_lua, _debug| Err(mlua::Error::RuntimeError("lua vm cycles exceeded".into())),
            );

            // Build the events table in Lua
            let result = (|| -> mlua::Result<()> {
                let lua = &player.lua;
                let events_table = lua.create_table()?;

                for (i, event) in events.iter().enumerate() {
                    let evt = lua.create_table()?;
                    match event {
                        GameEvent::CreatureSpawned { id, parent } => {
                            evt.set("type", 0i32)?; // CREATURE_SPAWNED
                            evt.set("id", *id)?;
                            evt.set("parent", *parent)?;
                        }
                        GameEvent::CreatureKilled { id, killer } => {
                            evt.set("type", 1i32)?; // CREATURE_KILLED
                            evt.set("id", *id)?;
                            evt.set("killer", *killer)?;
                        }
                        GameEvent::CreatureAttacked { id, attacker } => {
                            evt.set("type", 2i32)?; // CREATURE_ATTACKED
                            evt.set("id", *id)?;
                            evt.set("attacker", *attacker)?;
                        }
                        GameEvent::PlayerCreated { player_id: _ } => {
                            evt.set("type", 3i32)?; // PLAYER_CREATED
                        }
                    }
                    events_table.set(i + 1, evt)?; // Lua tables are 1-indexed
                }

                // Call player_think(events)
                let player_think: mlua::Function = lua.globals().get("player_think")?;
                let _: () = player_think.call(events_table)?;

                Ok(())
            })();

            // Remove instruction hook after execution
            player.lua.remove_hook();

            if let Err(e) = result {
                // Log the error but don't crash the game
                tracing::warn!(player_id = pid, "Lua error in player_think: {e}");
                let player = self.players.get_mut(&pid).unwrap();
                player.output.push(format!("Lua error: {e}"));
            }

            lua_api::clear_game_state(&self.players.get(&pid).unwrap().lua);

            // Collect print output
            if let Some(player) = self.players.get_mut(&pid) {
                let output = print_output.borrow();
                player.output.extend(output.iter().cloned());
            }
        }
    }

    /// Process all creatures for one tick: suicides, aging, state actions.
    fn process_creatures(&mut self, delta: i32) {
        // Collect creature IDs to process
        let creature_ids: Vec<u32> = self.creatures.borrow().keys().copied().collect();

        // Handle suicides first
        let suicides: Vec<u32> = creature_ids
            .iter()
            .filter(|id| {
                self.creatures
                    .borrow()
                    .get(id)
                    .map(|c| c.suicide)
                    .unwrap_or(false)
            })
            .copied()
            .collect();
        for id in suicides {
            self.kill_creature(id, Some(id));
        }

        // Re-collect after removals
        let creature_ids: Vec<u32> = self.creatures.borrow().keys().copied().collect();

        // Age all creatures, collect deaths
        let mut deaths = Vec::new();
        for &id in &creature_ids {
            let died = {
                let mut creatures = self.creatures.borrow_mut();
                if let Some(creature) = creatures.get_mut(&id) {
                    creature.do_age(delta)
                } else {
                    false
                }
            };
            if died {
                deaths.push(id);
            }
        }
        for id in deaths {
            self.kill_creature(id, None);
        }

        // Re-collect after deaths
        let creature_ids: Vec<u32> = self.creatures.borrow().keys().copied().collect();

        // Process state actions
        let mut new_spawns: Vec<(u32, u32, i32, i32, u8)> = Vec::new(); // (parent_id, player_id, x, y, type)
        let mut attack_events: Vec<(u32, u32, u32)> = Vec::new(); // (target_id, target_player, attacker_id)
        let mut kills_from_combat: Vec<(u32, u32)> = Vec::new(); // (creature_id, killer_id)

        for &id in &creature_ids {
            let mut creatures = self.creatures.borrow_mut();
            let creature = match creatures.get_mut(&id) {
                Some(c) => c,
                None => continue,
            };

            match creature.state {
                CREATURE_WALK => {
                    creature.do_walk(delta);
                }
                CREATURE_HEAL => {
                    let finished = creature.do_heal(delta);
                    if finished {
                        creature.set_state(CREATURE_IDLE);
                    }
                }
                CREATURE_EAT => {
                    let tx = creature.tile_x();
                    let ty = creature.tile_y();
                    let tile_food = self.world.borrow().get_food(tx, ty);
                    let (eaten, finished) = creature.do_eat(delta, tile_food);
                    if eaten > 0 {
                        self.world.borrow_mut().eat_food(tx, ty, eaten);
                    }
                    if finished {
                        creature.set_state(CREATURE_IDLE);
                    }
                }
                CREATURE_ATTACK => {
                    let target_id = match creature.target_id {
                        Some(tid) => tid,
                        None => {
                            creature.set_state(CREATURE_IDLE);
                            continue;
                        }
                    };

                    // Get target info
                    let attacker_type = creature.creature_type;
                    let attacker_x = creature.x;
                    let attacker_y = creature.y;
                    let attacker_id = creature.id;

                    let target = match creatures.get(&target_id) {
                        Some(t) => t,
                        None => {
                            let c = creatures.get_mut(&id).unwrap();
                            c.set_state(CREATURE_IDLE);
                            continue;
                        }
                    };

                    let target_type = target.creature_type;
                    let target_x = target.x;
                    let target_y = target.y;
                    let target_player = target.player_id;

                    let range = ATTACK_DISTANCE[attacker_type as usize][target_type as usize];
                    let damage_per_sec = HITPOINTS[attacker_type as usize][target_type as usize];

                    // Check range
                    let dx = (attacker_x - target_x) as i64;
                    let dy = (attacker_y - target_y) as i64;
                    let dist = ((dx * dx + dy * dy) as f64).sqrt() as i32;

                    if range == 0 || damage_per_sec == 0 || dist > range {
                        let c = creatures.get_mut(&id).unwrap();
                        c.set_state(CREATURE_IDLE);
                        continue;
                    }

                    // Apply damage
                    let damage = damage_per_sec * delta / 1000;
                    let target = creatures.get_mut(&target_id).unwrap();
                    target.health -= damage;

                    attack_events.push((target_id, target_player, attacker_id));

                    if target.health <= 0 {
                        kills_from_combat.push((target_id, attacker_id));
                        let c = creatures.get_mut(&id).unwrap();
                        c.set_state(CREATURE_IDLE);
                    }
                }
                CREATURE_CONVERT => {
                    let result = creature.do_convert(delta);
                    if result.is_some() {
                        creature.set_state(CREATURE_IDLE);
                    }
                }
                CREATURE_SPAWN => {
                    let player_id = creature.player_id;
                    let cx = creature.x;
                    let cy = creature.y;
                    let spawn_type_val = SPAWN_TYPE[creature.creature_type as usize];
                    let completed = creature.do_spawn(delta);
                    if completed && spawn_type_val >= 0 {
                        new_spawns.push((id, player_id, cx, cy, spawn_type_val as u8));
                        creature.set_state(CREATURE_IDLE);
                    }
                }
                CREATURE_FEED => {
                    let target_id = match creature.target_id {
                        Some(tid) => tid,
                        None => {
                            creature.set_state(CREATURE_IDLE);
                            continue;
                        }
                    };

                    let feeder_type = creature.creature_type;
                    let feeder_x = creature.x;
                    let feeder_y = creature.y;
                    let feeder_food = creature.food;

                    let feed_dist = FEED_DISTANCE[feeder_type as usize];
                    let feed_spd = FEED_SPEED[feeder_type as usize];

                    if feed_dist == 0 || feed_spd == 0 || feeder_food <= 0 {
                        creature.set_state(CREATURE_IDLE);
                        continue;
                    }

                    let target = match creatures.get(&target_id) {
                        Some(t) => t,
                        None => {
                            let c = creatures.get_mut(&id).unwrap();
                            c.set_state(CREATURE_IDLE);
                            continue;
                        }
                    };

                    let target_x = target.x;
                    let target_y = target.y;
                    let target_food = target.food;
                    let target_max_food = target.max_food();

                    let dx = (feeder_x - target_x) as i64;
                    let dy = (feeder_y - target_y) as i64;
                    let dist = ((dx * dx + dy * dy) as f64).sqrt() as i32;

                    if dist > feed_dist {
                        let c = creatures.get_mut(&id).unwrap();
                        c.set_state(CREATURE_IDLE);
                        continue;
                    }

                    let rate = feed_spd * delta / 1000;
                    let target_room = target_max_food - target_food;
                    let amount = rate.min(feeder_food).min(target_room);

                    if amount <= 0 {
                        let c = creatures.get_mut(&id).unwrap();
                        c.set_state(CREATURE_IDLE);
                        continue;
                    }

                    let feeder = creatures.get_mut(&id).unwrap();
                    feeder.food -= amount;
                    let feeder_food_left = feeder.food;

                    let target = creatures.get_mut(&target_id).unwrap();
                    target.food += amount;

                    if feeder_food_left <= 0 {
                        let c = creatures.get_mut(&id).unwrap();
                        c.set_state(CREATURE_IDLE);
                    }
                }
                _ => {
                    // Idle: do nothing
                }
            }
        }

        // Queue attack events
        for (target_id, target_player, attacker_id) in attack_events {
            self.pending_events.entry(target_player).or_default().push(
                GameEvent::CreatureAttacked {
                    id: target_id,
                    attacker: attacker_id,
                },
            );
        }

        // Process kills from combat
        for (creature_id, killer_id) in kills_from_combat {
            self.kill_creature(creature_id, Some(killer_id));
        }

        // Process new spawns
        for (parent_id, player_id, x, y, spawn_type) in new_spawns {
            self.spawn_offspring(parent_id, player_id, x, y, spawn_type);
        }
    }

    /// King of the Hill scoring: player holding the KOTH tile exclusively
    /// earns +30 points for every 10,000ms of continuous holding.
    fn process_koth(&mut self) {
        let world = self.world.borrow();
        let koth_x = world.koth_x;
        let koth_y = world.koth_y;
        drop(world);

        let creatures = self.creatures.borrow();
        let mut koth_player: Option<u32> = None;
        let mut multiple_players = false;

        for creature in creatures.values() {
            if creature.tile_x() == koth_x && creature.tile_y() == koth_y {
                match koth_player {
                    None => koth_player = Some(creature.player_id),
                    Some(pid) if pid != creature.player_id => {
                        multiple_players = true;
                        break;
                    }
                    _ => {}
                }
            }
        }
        drop(creatures);

        if multiple_players {
            // Contested: reset king
            self.king_player_id = None;
            self.king_time = 0;
        } else if let Some(pid) = koth_player {
            // Single player on KOTH
            if self.king_player_id != Some(pid) {
                // New king
                self.king_player_id = Some(pid);
                self.king_time = 0;
            }
            self.king_time += self.tick_delta;
            while self.king_time >= 10000 {
                self.change_player_score(pid, 30);
                self.king_time -= 10000;
            }
        } else {
            // No one on KOTH
            self.king_player_id = None;
            self.king_time = 0;
        }
    }

    /// Check if only one player has creatures remaining (win condition).
    /// Returns Some(player_id) if exactly one player has creatures, None otherwise.
    /// Also returns None if no players have creatures at all.
    pub fn check_winner(&self) -> Option<u32> {
        let creatures = self.creatures.borrow();
        let mut player_with_creatures: Option<u32> = None;
        for c in creatures.values() {
            match player_with_creatures {
                None => player_with_creatures = Some(c.player_id),
                Some(pid) if pid != c.player_id => return None, // multiple players alive
                _ => {}
            }
        }
        // Only return a winner if there are actually creatures (and more than 1 player in the game)
        if player_with_creatures.is_some() && self.players.len() > 1 {
            player_with_creatures
        } else {
            None
        }
    }

    /// Check if any player has reached the score limit.
    pub fn check_score_limit_winner(&self) -> Option<u32> {
        let limit = self.score_limit?;
        let scores = self.player_scores.borrow();
        for (&pid, &score) in scores.iter() {
            if score >= limit {
                return Some(pid);
            }
        }
        None
    }

    /// Create a snapshot of the game state for rendering.
    pub fn snapshot(&mut self) -> GameSnapshot {
        let creatures = self.creatures.borrow();
        let creature_snapshots: Vec<CreatureSnapshot> = creatures
            .values()
            .map(|c| CreatureSnapshot {
                id: c.id,
                x: c.x,
                y: c.y,
                creature_type: c.creature_type,
                health: c.health,
                max_health: c.max_health(),
                food: c.food,
                state: c.state,
                player_id: c.player_id,
                message: c.message.clone(),
                target_id: c.target_id,
            })
            .collect();

        let player_snapshots: Vec<PlayerSnapshot> = self
            .players
            .values_mut()
            .map(|p| PlayerSnapshot {
                id: p.id,
                name: p.name.clone(),
                score: p.score,
                color: p.color,
                num_creatures: p.num_creatures,
                output: std::mem::take(&mut p.output),
            })
            .collect();

        let events = std::mem::take(&mut self.broadcast_events);

        GameSnapshot {
            game_time: self.game_time,
            creatures: creature_snapshots,
            players: player_snapshots,
            king_player_id: self.king_player_id,
            events,
        }
    }

    /// Compute a delta between the current snapshot and a previous one.
    /// Only includes creatures that changed or were added/removed.
    pub fn compute_delta(current: &GameSnapshot, previous: &GameSnapshot) -> GameSnapshotDelta {
        use std::collections::HashMap;

        let prev_map: HashMap<u32, &CreatureSnapshot> =
            previous.creatures.iter().map(|c| (c.id, c)).collect();
        let curr_map: HashMap<u32, &CreatureSnapshot> =
            current.creatures.iter().map(|c| (c.id, c)).collect();

        let mut changed = Vec::new();
        let mut removed = Vec::new();

        // Find changed or new creatures
        for c in &current.creatures {
            match prev_map.get(&c.id) {
                Some(prev) if *prev == c => {} // unchanged
                _ => changed.push(c.clone()),  // new or changed
            }
        }

        // Find removed creatures
        for id in prev_map.keys() {
            if !curr_map.contains_key(id) {
                removed.push(*id);
            }
        }

        GameSnapshotDelta {
            game_time: current.game_time,
            changed,
            removed,
            players: current.players.clone(),
            king_player_id: current.king_player_id,
            events: current.events.clone(),
        }
    }

    /// Create a snapshot of the world (tiles) for initial WebSocket handshake.
    pub fn world_snapshot(&self) -> WorldSnapshot {
        let world = self.world.borrow();
        let mut tiles = Vec::with_capacity(world.width * world.height);
        for y in 0..world.height {
            for x in 0..world.width {
                tiles.push(TileSnapshot {
                    x,
                    y,
                    food: world.get_food(x, y),
                    tile_type: world.get_type(x, y),
                    gfx: world.get_gfx(x, y),
                });
            }
        }
        WorldSnapshot {
            width: world.width,
            height: world.height,
            koth_x: world.koth_x,
            koth_y: world.koth_y,
            tiles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a simple open world for testing.
    fn make_test_world() -> World {
        let mut w = World::new(10, 10);
        for y in 1..9 {
            for x in 1..9 {
                w.set_type(x, y, TILE_PLAIN);
            }
        }
        // Add some food
        w.add_food(3, 3, 5000);
        w
    }

    #[test]
    fn test_new_game() {
        let world = make_test_world();
        let game = Game::new(world);
        assert_eq!(game.game_time, 0);
        assert!(game.players.is_empty());
        assert!(game.creatures.borrow().is_empty());
    }

    #[test]
    fn test_add_player() {
        let world = make_test_world();
        let mut game = Game::new(world);

        let pid = game.add_player("TestBot", "");
        assert!(pid.is_ok());
        let pid = pid.unwrap();
        assert_eq!(pid, 1);
        assert!(game.players.contains_key(&pid));
        assert_eq!(game.players.get(&pid).unwrap().name, "TestBot");
    }

    #[test]
    fn test_spawn_creature() {
        let world = make_test_world();
        let mut game = Game::new(world);
        let pid = game.add_player("TestBot", "").unwrap();

        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        let cid = game.spawn_creature(pid, cx, cy, CREATURE_SMALL);
        assert!(cid.is_some());
        let cid = cid.unwrap();

        let creatures = game.creatures.borrow();
        let creature = creatures.get(&cid).unwrap();
        assert_eq!(creature.x, cx);
        assert_eq!(creature.y, cy);
        assert_eq!(creature.creature_type, CREATURE_SMALL);
        assert_eq!(creature.player_id, pid);
    }

    #[test]
    fn test_basic_tick() {
        let world = make_test_world();
        let mut game = Game::new(world);
        let pid = game.add_player("TestBot", "").unwrap();

        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        let cid = game.spawn_creature(pid, cx, cy, CREATURE_SMALL).unwrap();

        let health_before = game.creatures.borrow().get(&cid).unwrap().health;

        // Run one tick
        game.tick();

        // After tick, creature should have aged
        let health_after = game.creatures.borrow().get(&cid).unwrap().health;
        assert!(health_after < health_before);
        assert_eq!(game.game_time, 100);
    }

    #[test]
    fn test_creature_eating() {
        let world = make_test_world();
        let mut game = Game::new(world);
        // Bot code that sets eating state
        let code = r#"
            function Creature:main()
                self:begin_eating()
                self:wait_for_next_round()
            end
        "#;
        let pid = game.add_player("EatBot", code).unwrap();

        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        let cid = game.spawn_creature(pid, cx, cy, CREATURE_SMALL).unwrap();

        // Run several ticks (first tick processes spawn event and starts coroutine,
        // subsequent ticks resume coroutine which sets eating)
        for _ in 0..5 {
            game.tick();
        }

        let creatures = game.creatures.borrow();
        let creature = creatures.get(&cid).unwrap();
        // Creature should have eaten some food
        assert!(
            creature.food > 0,
            "Creature food should be > 0 after eating, got {}",
            creature.food
        );
    }

    #[test]
    fn test_creature_walking() {
        let world = make_test_world();
        let mut game = Game::new(world);

        let target_x = World::tile_center(6);
        let target_y = World::tile_center(3);
        let code = format!(
            r#"
            function Creature:main()
                self:set_path({target_x}, {target_y})
                self:begin_walk_path()
                while self:is_walking() do
                    self:wait_for_next_round()
                end
            end
            "#
        );
        let pid = game.add_player("WalkBot", &code).unwrap();

        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        let cid = game.spawn_creature(pid, cx, cy, CREATURE_SMALL).unwrap();

        // Run several ticks
        for _ in 0..20 {
            game.tick();
        }

        let creatures = game.creatures.borrow();
        let creature = creatures.get(&cid).unwrap();
        // Creature should have moved closer to target
        let start_dist = ((cx - target_x).abs() + (cy - target_y).abs()) as i32;
        let end_dist = ((creature.x - target_x).abs() + (creature.y - target_y).abs()) as i32;
        assert!(
            end_dist < start_dist,
            "Creature should have moved closer to target"
        );
    }

    #[test]
    fn test_combat() {
        let world = make_test_world();
        let mut game = Game::new(world);

        // Player 1: big creature that attacks
        let pid1 = game.add_player("Attacker", "").unwrap();
        // Player 2: small creature as target
        let pid2 = game.add_player("Target", "").unwrap();

        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        let attacker_id = game.spawn_creature(pid1, cx, cy, CREATURE_BIG).unwrap();
        let target_id = game
            .spawn_creature(pid2, cx + 100, cy, CREATURE_SMALL)
            .unwrap();

        // Manually set the attacker to attack the target
        {
            let mut creatures = game.creatures.borrow_mut();
            let attacker = creatures.get_mut(&attacker_id).unwrap();
            attacker.set_target(target_id);
            attacker.set_state(CREATURE_ATTACK);
        }

        let target_health_before = game.creatures.borrow().get(&target_id).unwrap().health;

        // Run one tick (skip player_think by not having bot code do anything)
        game.tick();

        // Target should have taken damage
        let creatures = game.creatures.borrow();
        if let Some(target) = creatures.get(&target_id) {
            assert!(
                target.health < target_health_before,
                "Target should have taken damage"
            );
        }
        // (target might also be dead if damage is high enough)
    }

    #[test]
    fn test_koth() {
        let world = make_test_world();
        let mut game = Game::new(world);
        game.score_limit = None; // disable score limit for this test
        let pid = game.add_player("KothBot", "").unwrap();

        // Spawn creature on the koth tile
        let koth_x = World::tile_center(game.world.borrow().koth_x);
        let koth_y = World::tile_center(game.world.borrow().koth_y);
        game.spawn_creature(pid, koth_x, koth_y, CREATURE_SMALL);

        // Run a single tick -- king is set but no score yet (need 10,000ms)
        game.tick();
        assert_eq!(game.king_player_id, Some(pid));
        let score_after_1 = *game.player_scores.borrow().get(&pid).unwrap_or(&0);
        assert_eq!(score_after_1, 0, "No KOTH score after 1 tick (100ms)");
        assert_eq!(game.king_time, 100);

        // Run 99 more ticks (total 10,000ms) -- should award +30
        for _ in 0..99 {
            game.tick();
        }
        let score_after_100 = *game.player_scores.borrow().get(&pid).unwrap_or(&0);
        assert_eq!(
            score_after_100, 30,
            "Should have +30 after 10,000ms on KOTH"
        );
        assert_eq!(game.king_time, 0);
        assert_eq!(game.king_player_id, Some(pid));
    }

    #[test]
    fn test_remove_player() {
        let world = make_test_world();
        let mut game = Game::new(world);
        let pid = game.add_player("TestBot", "").unwrap();
        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        game.spawn_creature(pid, cx, cy, CREATURE_SMALL);

        assert_eq!(game.creatures.borrow().len(), 1);
        game.remove_player(pid);
        assert!(!game.players.contains_key(&pid));
        assert_eq!(game.creatures.borrow().len(), 0);
    }

    #[test]
    fn test_snapshot() {
        let world = make_test_world();
        let mut game = Game::new(world);
        let pid = game.add_player("TestBot", "").unwrap();
        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        game.spawn_creature(pid, cx, cy, CREATURE_SMALL);

        let snap = game.snapshot();
        assert_eq!(snap.game_time, 0);
        assert_eq!(snap.creatures.len(), 1);
        assert_eq!(snap.creatures[0].x, cx);
    }

    #[test]
    fn test_check_winner_no_winner() {
        let world = make_test_world();
        let mut game = Game::new(world);
        let pid1 = game.add_player("Bot1", "").unwrap();
        let pid2 = game.add_player("Bot2", "").unwrap();

        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        game.spawn_creature(pid1, cx, cy, CREATURE_SMALL);
        game.spawn_creature(pid2, cx + 256, cy, CREATURE_SMALL);

        // Both players have creatures — no winner
        assert_eq!(game.check_winner(), None);
    }

    #[test]
    fn test_check_winner_one_player_left() {
        let world = make_test_world();
        let mut game = Game::new(world);
        let pid1 = game.add_player("Bot1", "").unwrap();
        let _pid2 = game.add_player("Bot2", "").unwrap();

        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        game.spawn_creature(pid1, cx, cy, CREATURE_SMALL);
        // pid2 has no creatures

        assert_eq!(game.check_winner(), Some(pid1));
    }

    #[test]
    fn test_check_winner_single_player_no_win() {
        let world = make_test_world();
        let mut game = Game::new(world);
        let pid1 = game.add_player("Bot1", "").unwrap();

        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        game.spawn_creature(pid1, cx, cy, CREATURE_SMALL);

        // Only 1 player in game — no win condition
        assert_eq!(game.check_winner(), None);
    }

    #[test]
    fn test_instruction_limit_infinite_loop() {
        let world = make_test_world();
        let mut game = Game::new(world);

        // Bot with an infinite loop in main()
        let code = r#"
            function Creature:main()
                while true do end
            end
        "#;
        let pid = game.add_player("InfiniteBot", code).unwrap();
        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        game.spawn_creature(pid, cx, cy, CREATURE_SMALL);

        // This should NOT hang — the instruction limit should catch it
        game.tick();
        assert_eq!(game.game_time, 100);

        // Error should appear in player output
        let snap = game.snapshot();
        let player = snap.players.iter().find(|p| p.id == pid).unwrap();
        let has_error = player
            .output
            .iter()
            .any(|line| line.contains("cycles exceeded"));
        assert!(
            has_error,
            "Expected 'cycles exceeded' error in output, got: {:?}",
            player.output
        );
    }

    #[test]
    fn test_instruction_limit_loop_in_onspawned() {
        let world = make_test_world();
        let mut game = Game::new(world);

        // Bot with an infinite loop at load time / onSpawned
        let code = r#"
            function Creature:onSpawned()
                while true do end
            end
            function Creature:main()
                self:wait_for_next_round()
            end
        "#;
        let pid = game.add_player("SpawnLoopBot", code).unwrap();
        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        game.spawn_creature(pid, cx, cy, CREATURE_SMALL);

        // Should not hang
        game.tick();
        assert_eq!(game.game_time, 100);
    }

    #[test]
    fn test_delta_compression_no_change() {
        let snap = GameSnapshot {
            game_time: 100,
            creatures: vec![CreatureSnapshot {
                id: 1,
                x: 500,
                y: 500,
                creature_type: 0,
                health: 10000,
                max_health: 10000,
                food: 0,
                state: 0,
                player_id: 1,
                message: String::new(),
                target_id: None,
            }],
            players: vec![],
            king_player_id: None,
            events: vec![],
        };

        let delta = Game::compute_delta(&snap, &snap);
        assert!(delta.changed.is_empty(), "No creatures should have changed");
        assert!(delta.removed.is_empty(), "No creatures should be removed");
    }

    #[test]
    fn test_delta_compression_creature_moved() {
        let prev = GameSnapshot {
            game_time: 100,
            creatures: vec![CreatureSnapshot {
                id: 1,
                x: 500,
                y: 500,
                creature_type: 0,
                health: 10000,
                max_health: 10000,
                food: 0,
                state: 0,
                player_id: 1,
                message: String::new(),
                target_id: None,
            }],
            players: vec![],
            king_player_id: None,
            events: vec![],
        };

        let mut current = prev.clone();
        current.game_time = 200;
        current.creatures[0].x = 600; // moved

        let delta = Game::compute_delta(&current, &prev);
        assert_eq!(delta.changed.len(), 1);
        assert_eq!(delta.changed[0].id, 1);
        assert_eq!(delta.changed[0].x, 600);
        assert!(delta.removed.is_empty());
    }

    #[test]
    fn test_delta_compression_creature_added() {
        let prev = GameSnapshot {
            game_time: 100,
            creatures: vec![CreatureSnapshot {
                id: 1,
                x: 500,
                y: 500,
                creature_type: 0,
                health: 10000,
                max_health: 10000,
                food: 0,
                state: 0,
                player_id: 1,
                message: String::new(),
                target_id: None,
            }],
            players: vec![],
            king_player_id: None,
            events: vec![],
        };

        let mut current = prev.clone();
        current.game_time = 200;
        current.creatures.push(CreatureSnapshot {
            id: 2,
            x: 700,
            y: 700,
            creature_type: 1,
            health: 20000,
            max_health: 20000,
            food: 0,
            state: 0,
            player_id: 2,
            message: String::new(),
            target_id: None,
        });

        let delta = Game::compute_delta(&current, &prev);
        assert_eq!(delta.changed.len(), 1); // only the new creature (id=2)
        assert_eq!(delta.changed[0].id, 2);
        assert!(delta.removed.is_empty());
    }

    #[test]
    fn test_delta_compression_creature_removed() {
        let prev = GameSnapshot {
            game_time: 100,
            creatures: vec![
                CreatureSnapshot {
                    id: 1,
                    x: 500,
                    y: 500,
                    creature_type: 0,
                    health: 10000,
                    max_health: 10000,
                    food: 0,
                    state: 0,
                    player_id: 1,
                    message: String::new(),
                    target_id: None,
                },
                CreatureSnapshot {
                    id: 2,
                    x: 700,
                    y: 700,
                    creature_type: 1,
                    health: 20000,
                    max_health: 20000,
                    food: 0,
                    state: 0,
                    player_id: 2,
                    message: String::new(),
                    target_id: None,
                },
            ],
            players: vec![],
            king_player_id: None,
            events: vec![],
        };

        let mut current = prev.clone();
        current.game_time = 200;
        current.creatures.retain(|c| c.id != 2); // remove creature 2

        let delta = Game::compute_delta(&current, &prev);
        assert!(delta.changed.is_empty());
        assert_eq!(delta.removed.len(), 1);
        assert_eq!(delta.removed[0], 2);
    }

    #[test]
    fn test_tick_records_timings() {
        let world = make_test_world();
        let mut game = Game::new(world);
        let pid = game.add_player("TestBot", "").unwrap();
        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        game.spawn_creature(pid, cx, cy, CREATURE_SMALL);

        game.tick();

        // Timings should be recorded (total_us > 0 since some work was done)
        assert!(
            game.last_tick_timings.total_us > 0,
            "Total tick time should be > 0"
        );
    }

    #[test]
    fn test_spatial_index_used_in_tick() {
        // Verify that the spatial index is rebuilt each tick without errors
        let world = make_test_world();
        let mut game = Game::new(world);
        let pid1 = game.add_player("Bot1", "").unwrap();
        let pid2 = game.add_player("Bot2", "").unwrap();

        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        game.spawn_creature(pid1, cx, cy, CREATURE_SMALL);
        game.spawn_creature(pid2, cx + 256, cy, CREATURE_SMALL);

        // Run several ticks -- the spatial index is rebuilt each time
        for _ in 0..10 {
            game.tick();
        }
        assert_eq!(game.game_time, 1000);
    }
}
