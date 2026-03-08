use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use mlua::{Lua, MultiValue, Result as LuaResult, Value};

use super::config::*;
use super::creature::Creature;
use super::spatial::SpatialGrid;
use super::world::World;

/// Shared game state accessible during Lua execution.
/// Stored as Lua app_data during player_think, then removed afterward.
pub struct LuaGameState {
    pub world: Rc<RefCell<World>>,
    pub creatures: Rc<RefCell<HashMap<u32, Creature>>>,
    pub game_time: i64,
    pub player_id: u32,
    pub player_scores: Rc<RefCell<HashMap<u32, i32>>>,
    pub player_names: Rc<RefCell<HashMap<u32, String>>>,
    pub king_player_id: Option<u32>,
    pub print_output: Rc<RefCell<Vec<String>>>,
    /// Optional spatial index for fast nearest-enemy queries.
    pub spatial_grid: Option<Rc<RefCell<SpatialGrid>>>,
}

/// Register all Lua constants into the VM.
pub fn register_constants(lua: &Lua, player_id: u32) -> LuaResult<()> {
    let g = lua.globals();

    // Creature types
    g.set("CREATURE_SMALL", CREATURE_SMALL as i32)?;
    g.set("CREATURE_BIG", CREATURE_BIG as i32)?;
    g.set("CREATURE_FLYER", CREATURE_FLYER as i32)?;

    // Creature states
    g.set("CREATURE_IDLE", CREATURE_IDLE as i32)?;
    g.set("CREATURE_WALK", CREATURE_WALK as i32)?;
    g.set("CREATURE_HEAL", CREATURE_HEAL as i32)?;
    g.set("CREATURE_EAT", CREATURE_EAT as i32)?;
    g.set("CREATURE_ATTACK", CREATURE_ATTACK as i32)?;
    g.set("CREATURE_CONVERT", CREATURE_CONVERT as i32)?;
    g.set("CREATURE_SPAWN", CREATURE_SPAWN as i32)?;
    g.set("CREATURE_FEED", CREATURE_FEED as i32)?;

    // Tile types
    g.set("TILE_SOLID", TILE_SOLID as i32)?;
    g.set("TILE_PLAIN", TILE_PLAIN as i32)?;

    // Tile GFX
    g.set("TILE_GFX_SOLID", TILE_GFX_SOLID as i32)?;
    g.set("TILE_GFX_PLAIN", TILE_GFX_PLAIN as i32)?;
    g.set("TILE_GFX_BORDER", TILE_GFX_BORDER as i32)?;
    g.set("TILE_GFX_SNOW_SOLID", TILE_GFX_SNOW_SOLID as i32)?;
    g.set("TILE_GFX_SNOW_PLAIN", TILE_GFX_SNOW_PLAIN as i32)?;
    g.set("TILE_GFX_SNOW_BORDER", TILE_GFX_SNOW_BORDER as i32)?;
    g.set("TILE_GFX_WATER", TILE_GFX_WATER as i32)?;
    g.set("TILE_GFX_LAVA", TILE_GFX_LAVA as i32)?;
    g.set("TILE_GFX_NONE", TILE_GFX_NONE as i32)?;
    g.set("TILE_GFX_KOTH", TILE_GFX_KOTH as i32)?;
    g.set("TILE_GFX_DESERT", TILE_GFX_DESERT as i32)?;

    // Tile size (TILE_WIDTH and TILE_HEIGHT in original)
    g.set("TILE_WIDTH", TILE_SIZE)?;
    g.set("TILE_HEIGHT", TILE_SIZE)?;

    // Event types (matching original enum order in player.h)
    g.set("CREATURE_SPAWNED", 0i32)?;
    g.set("CREATURE_KILLED", 1i32)?;
    g.set("CREATURE_ATTACKED", 2i32)?;
    g.set("PLAYER_CREATED", 3i32)?;

    // Player number
    g.set("player_number", player_id)?;

    Ok(())
}

/// Helper: get LuaGameState from Lua app_data or return Lua error.
fn get_game_state(lua: &Lua) -> LuaResult<Rc<RefCell<LuaGameState>>> {
    lua.app_data_ref::<Rc<RefCell<LuaGameState>>>()
        .map(|r| r.clone())
        .ok_or_else(|| mlua::Error::runtime("Game state not available (not in think phase)"))
}

/// Helper: check creature ownership
fn check_ownership(
    creatures: &HashMap<u32, Creature>,
    creature_id: u32,
    player_id: u32,
) -> LuaResult<()> {
    match creatures.get(&creature_id) {
        Some(c) if c.player_id == player_id => Ok(()),
        Some(_) => Err(mlua::Error::runtime(format!(
            "Creature {creature_id} does not belong to player {player_id}"
        ))),
        None => Err(mlua::Error::runtime(format!(
            "Creature {creature_id} does not exist"
        ))),
    }
}

/// Register all bot-facing API functions into the Lua VM.
pub fn register_functions(lua: &Lua, _player_id: u32) -> LuaResult<()> {
    let g = lua.globals();

    // set_path(creature_id, x, y) -> bool
    g.set(
        "set_path",
        lua.create_function(|lua, (creature_id, x, y): (u32, i32, i32)| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let player_id = gs.player_id;
            let mut creatures = gs.creatures.borrow_mut();
            let world = gs.world.borrow();
            check_ownership(&creatures, creature_id, player_id)?;

            let creature = creatures.get(&creature_id).unwrap();
            let path = world.find_path(creature.x, creature.y, x, y);
            match path {
                Some(waypoints) => {
                    let creature = creatures.get_mut(&creature_id).unwrap();
                    creature.set_path(waypoints);
                    Ok(true)
                }
                None => Ok(false),
            }
        })?,
    )?;

    // set_state(creature_id, state) -> bool
    g.set(
        "set_state",
        lua.create_function(|lua, (creature_id, state): (u32, u8)| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let player_id = gs.player_id;
            let mut creatures = gs.creatures.borrow_mut();
            check_ownership(&creatures, creature_id, player_id)?;
            let creature = creatures.get_mut(&creature_id).unwrap();
            Ok(creature.set_state(state))
        })?,
    )?;

    // get_state(creature_id) -> number
    g.set(
        "get_state",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let creature = creatures
                .get(&creature_id)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {creature_id} not found")))?;
            Ok(creature.state as i32)
        })?,
    )?;

    // set_target(creature_id, target_id) -> bool
    g.set(
        "set_target",
        lua.create_function(|lua, (creature_id, target_id): (u32, u32)| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let player_id = gs.player_id;
            let mut creatures = gs.creatures.borrow_mut();
            check_ownership(&creatures, creature_id, player_id)?;
            let creature = creatures.get_mut(&creature_id).unwrap();
            Ok(creature.set_target(target_id))
        })?,
    )?;

    // set_convert(creature_id, type) -> bool
    g.set(
        "set_convert",
        lua.create_function(|lua, (creature_id, target_type): (u32, u8)| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let player_id = gs.player_id;
            let mut creatures = gs.creatures.borrow_mut();
            check_ownership(&creatures, creature_id, player_id)?;
            let creature = creatures.get_mut(&creature_id).unwrap();
            Ok(creature.set_conversion_type(target_type))
        })?,
    )?;

    // suicide(creature_id)
    g.set(
        "suicide",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let player_id = gs.player_id;
            let mut creatures = gs.creatures.borrow_mut();
            check_ownership(&creatures, creature_id, player_id)?;
            let creature = creatures.get_mut(&creature_id).unwrap();
            creature.suicide = true;
            Ok(())
        })?,
    )?;

    // get_pos(creature_id) -> x, y
    g.set(
        "get_pos",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let creature = creatures
                .get(&creature_id)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {creature_id} not found")))?;
            Ok((creature.x, creature.y))
        })?,
    )?;

    // get_type(creature_id) -> number
    g.set(
        "get_type",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let creature = creatures
                .get(&creature_id)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {creature_id} not found")))?;
            Ok(creature.creature_type as i32)
        })?,
    )?;

    // get_food(creature_id) -> number
    g.set(
        "get_food",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let creature = creatures
                .get(&creature_id)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {creature_id} not found")))?;
            Ok(creature.food)
        })?,
    )?;

    // get_health(creature_id) -> number (percentage 0-100!)
    g.set(
        "get_health",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let creature = creatures
                .get(&creature_id)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {creature_id} not found")))?;
            Ok(creature.health_percent())
        })?,
    )?;

    // get_speed(creature_id) -> number
    g.set(
        "get_speed",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let creature = creatures
                .get(&creature_id)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {creature_id} not found")))?;
            Ok(creature.speed())
        })?,
    )?;

    // get_tile_food(creature_id) -> number
    g.set(
        "get_tile_food",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let creature = creatures
                .get(&creature_id)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {creature_id} not found")))?;
            let world = gs.world.borrow();
            Ok(world.get_food(creature.tile_x(), creature.tile_y()))
        })?,
    )?;

    // get_tile_type(creature_id) -> number
    g.set(
        "get_tile_type",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let creature = creatures
                .get(&creature_id)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {creature_id} not found")))?;
            let world = gs.world.borrow();
            Ok(world.get_type(creature.tile_x(), creature.tile_y()) as i32)
        })?,
    )?;

    // get_max_food(creature_id) -> number
    g.set(
        "get_max_food",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let creature = creatures
                .get(&creature_id)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {creature_id} not found")))?;
            Ok(creature.max_food())
        })?,
    )?;

    // get_distance(creature_id1, creature_id2) -> number
    g.set(
        "get_distance",
        lua.create_function(|lua, (id1, id2): (u32, u32)| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let c1 = creatures
                .get(&id1)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {id1} not found")))?;
            let c2 = creatures
                .get(&id2)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {id2} not found")))?;
            Ok(c1.distance_to(c2.x, c2.y))
        })?,
    )?;

    // get_nearest_enemy(creature_id) -> id, x, y, player_num, distance (or nil)
    g.set(
        "get_nearest_enemy",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let creature = creatures
                .get(&creature_id)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {creature_id} not found")))?;
            let my_player = creature.player_id;
            let my_x = creature.x;
            let my_y = creature.y;

            // Use spatial index if available (O(n*k) instead of O(n^2))
            let nearest = if let Some(ref grid_rc) = gs.spatial_grid {
                let grid = grid_rc.borrow();
                grid.find_nearest_enemy(my_x, my_y, my_player)
            } else {
                // Fallback: linear scan (for backward compatibility)
                let mut best: Option<(u32, i32, i32, u32, i32)> = None;
                let mut min_dist = i32::MAX;

                for (_, other) in creatures.iter() {
                    if other.player_id == my_player {
                        continue;
                    }
                    let dx = (my_x - other.x) as i64;
                    let dy = (my_y - other.y) as i64;
                    let dist = ((dx * dx + dy * dy) as f64).sqrt() as i32;
                    if dist < min_dist {
                        min_dist = dist;
                        best = Some((other.id, other.x, other.y, other.player_id, dist));
                    }
                }
                best
            };

            match nearest {
                Some((id, x, y, player, dist)) => Ok(MultiValue::from_vec(vec![
                    Value::Integer(id as i64),
                    Value::Integer(x as i64),
                    Value::Integer(y as i64),
                    Value::Integer(player as i64),
                    Value::Integer(dist as i64),
                ])),
                None => Ok(MultiValue::new()),
            }
        })?,
    )?;

    // set_message(creature_id, msg)
    g.set(
        "set_message",
        lua.create_function(|lua, (creature_id, msg): (u32, String)| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let player_id = gs.player_id;
            let mut creatures = gs.creatures.borrow_mut();
            check_ownership(&creatures, creature_id, player_id)?;
            let creature = creatures.get_mut(&creature_id).unwrap();
            creature.set_message(&msg);
            Ok(())
        })?,
    )?;

    // creature_exists(creature_id) -> bool
    g.set(
        "creature_exists",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            Ok(creatures.contains_key(&creature_id))
        })?,
    )?;

    // creature_player(creature_id) -> number
    g.set(
        "creature_player",
        lua.create_function(|lua, creature_id: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let creatures = gs.creatures.borrow();
            let creature = creatures
                .get(&creature_id)
                .ok_or_else(|| mlua::Error::runtime(format!("Creature {creature_id} not found")))?;
            Ok(creature.player_id)
        })?,
    )?;

    // world_size() -> x1, y1, x2, y2 (pixel coords)
    g.set(
        "world_size",
        lua.create_function(|lua, ()| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let world = gs.world.borrow();
            let (x1, y1, x2, y2) = world.world_size_pixels();
            Ok((x1, y1, x2, y2))
        })?,
    )?;

    // game_time() -> number (milliseconds)
    g.set(
        "game_time",
        lua.create_function(|lua, ()| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            Ok(gs.game_time)
        })?,
    )?;

    // get_koth_pos() -> x, y (pixel coords of center)
    g.set(
        "get_koth_pos",
        lua.create_function(|lua, ()| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let world = gs.world.borrow();
            let (kx, ky) = world.koth_center_pixels();
            Ok((kx, ky))
        })?,
    )?;

    // player_exists(player_id) -> bool
    g.set(
        "player_exists",
        lua.create_function(|lua, pid: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let names = gs.player_names.borrow();
            Ok(names.contains_key(&pid))
        })?,
    )?;

    // king_player() -> number or nil
    g.set(
        "king_player",
        lua.create_function(|lua, ()| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            match gs.king_player_id {
                Some(id) => Ok(Value::Integer(id as i64)),
                None => Ok(Value::Nil),
            }
        })?,
    )?;

    // player_score(player_id) -> number
    g.set(
        "player_score",
        lua.create_function(|lua, pid: u32| {
            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            let scores = gs.player_scores.borrow();
            Ok(*scores.get(&pid).unwrap_or(&0))
        })?,
    )?;

    // get_cpu_usage() -> number (stub: always 0)
    g.set("get_cpu_usage", lua.create_function(|_, ()| Ok(0i32))?)?;

    // print(...) -> captures output
    g.set(
        "print",
        lua.create_function(|lua, args: MultiValue| {
            let mut parts = Vec::new();
            for val in args.iter() {
                match val {
                    Value::Nil => parts.push("nil".to_string()),
                    Value::Boolean(b) => parts.push(b.to_string()),
                    Value::Integer(n) => parts.push(n.to_string()),
                    Value::Number(n) => parts.push(n.to_string()),
                    Value::String(s) => {
                        parts.push(s.to_str().map(|s| s.to_string()).unwrap_or_default())
                    }
                    other => parts.push(format!("{:?}", other)),
                }
            }
            let line = parts.join("\t");

            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            gs.print_output.borrow_mut().push(line);
            Ok(())
        })?,
    )?;

    // client_print - alias for print (used by player.lua bootstrap)
    // We make it the same as our print
    g.set(
        "client_print",
        lua.create_function(|lua, args: MultiValue| {
            let mut parts = Vec::new();
            for val in args.iter() {
                match val {
                    Value::Nil => parts.push("nil".to_string()),
                    Value::Boolean(b) => parts.push(b.to_string()),
                    Value::Integer(n) => parts.push(n.to_string()),
                    Value::Number(n) => parts.push(n.to_string()),
                    Value::String(s) => {
                        parts.push(s.to_str().map(|s| s.to_string()).unwrap_or_default())
                    }
                    other => parts.push(format!("{:?}", other)),
                }
            }
            let line = parts.join("\t");

            let gs_rc = get_game_state(lua)?;
            let gs = gs_rc.borrow();
            gs.print_output.borrow_mut().push(line);
            Ok(())
        })?,
    )?;

    // creature_get_config(key) -> value (stub for creature_config metatable)
    g.set(
        "creature_get_config",
        lua.create_function(|_, key: String| -> LuaResult<Value> {
            // Return known config values. For MVP, return common ones.
            let _key_str = key.as_str();
            // Format: type_stat, e.g. "0_max_health"
            let val: Option<i32> = None;
            match val {
                Some(v) => Ok(Value::Integer(v as i64)),
                None => Ok(Value::Nil),
            }
        })?,
    )?;

    Ok(())
}

/// Set the game state into a Lua VM's app_data for the duration of a think call.
pub fn set_game_state(lua: &Lua, state: Rc<RefCell<LuaGameState>>) {
    lua.set_app_data(state);
}

/// Remove the game state from a Lua VM's app_data after a think call.
pub fn clear_game_state(lua: &Lua) {
    lua.remove_app_data::<Rc<RefCell<LuaGameState>>>();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a test Lua VM with API registered and game state set
    fn setup_test_lua() -> (Lua, Rc<RefCell<LuaGameState>>) {
        let lua = Lua::new();
        register_constants(&lua, 1).unwrap();
        register_functions(&lua, 1).unwrap();

        let mut world = World::new(10, 8);
        // Make some tiles walkable
        for y in 1..7 {
            for x in 1..9 {
                world.set_type(x, y, TILE_PLAIN);
            }
        }
        world.add_food(2, 2, 500);

        let mut creatures = HashMap::new();
        let c = Creature::new(100, 512, 512, CREATURE_SMALL, 1); // tile (2,2)
        creatures.insert(100, c);

        let mut player_scores = HashMap::new();
        player_scores.insert(1, 42);

        let mut player_names = HashMap::new();
        player_names.insert(1, "TestPlayer".to_string());

        let gs = Rc::new(RefCell::new(LuaGameState {
            world: Rc::new(RefCell::new(world)),
            creatures: Rc::new(RefCell::new(creatures)),
            game_time: 5000,
            player_id: 1,
            player_scores: Rc::new(RefCell::new(player_scores)),
            player_names: Rc::new(RefCell::new(player_names)),
            king_player_id: None,
            print_output: Rc::new(RefCell::new(Vec::new())),
            spatial_grid: None,
        }));

        set_game_state(&lua, gs.clone());
        (lua, gs)
    }

    #[test]
    fn test_lua_world_size() {
        let (lua, _gs) = setup_test_lua();
        let result: (i32, i32, i32, i32) = lua.load("return world_size()").eval().unwrap();
        // World is 10x8
        assert_eq!(result.0, TILE_SIZE); // 256
        assert_eq!(result.1, TILE_SIZE); // 256
        assert_eq!(result.2, 8 * TILE_SIZE); // (10-2)*256 = 2048
        assert_eq!(result.3, 6 * TILE_SIZE); // (8-2)*256 = 1536
    }

    #[test]
    fn test_lua_game_time() {
        let (lua, _gs) = setup_test_lua();
        let result: i64 = lua.load("return game_time()").eval().unwrap();
        assert_eq!(result, 5000);
    }

    #[test]
    fn test_lua_get_koth_pos() {
        let (lua, _gs) = setup_test_lua();
        let result: (i32, i32) = lua.load("return get_koth_pos()").eval().unwrap();
        // World 10x8, koth at (5,4), center = 5*256+128, 4*256+128
        assert_eq!(result.0, 5 * 256 + 128);
        assert_eq!(result.1, 4 * 256 + 128);
    }

    #[test]
    fn test_lua_creature_functions() {
        let (lua, _gs) = setup_test_lua();

        // get_pos
        let (x, y): (i32, i32) = lua.load("return get_pos(100)").eval().unwrap();
        assert_eq!(x, 512);
        assert_eq!(y, 512);

        // get_type
        let t: i32 = lua.load("return get_type(100)").eval().unwrap();
        assert_eq!(t, CREATURE_SMALL as i32);

        // get_health (percentage)
        let h: i32 = lua.load("return get_health(100)").eval().unwrap();
        assert_eq!(h, 100); // full health

        // get_speed
        let s: i32 = lua.load("return get_speed(100)").eval().unwrap();
        assert_eq!(s, 825); // small at full health

        // get_food
        let f: i32 = lua.load("return get_food(100)").eval().unwrap();
        assert_eq!(f, 0);

        // get_max_food
        let mf: i32 = lua.load("return get_max_food(100)").eval().unwrap();
        assert_eq!(mf, 10000);

        // get_tile_food (creature at tile 2,2 which has 500 food)
        let tf: i32 = lua.load("return get_tile_food(100)").eval().unwrap();
        assert_eq!(tf, 500);

        // get_tile_type
        let tt: i32 = lua.load("return get_tile_type(100)").eval().unwrap();
        assert_eq!(tt, TILE_PLAIN as i32);

        // creature_exists
        let exists: bool = lua.load("return creature_exists(100)").eval().unwrap();
        assert!(exists);
        let exists: bool = lua.load("return creature_exists(999)").eval().unwrap();
        assert!(!exists);

        // creature_player
        let pid: u32 = lua.load("return creature_player(100)").eval().unwrap();
        assert_eq!(pid, 1);
    }

    #[test]
    fn test_lua_set_path() {
        let (lua, gs) = setup_test_lua();
        // Set path to a walkable location
        let target_x = World::tile_center(5);
        let target_y = World::tile_center(3);
        let code = format!("return set_path(100, {target_x}, {target_y})");
        let result: bool = lua.load(&code).eval().unwrap();
        assert!(result);

        // Verify the creature now has a path
        let gs_inner = gs.borrow();
        let creatures = gs_inner.creatures.borrow();
        let creature = creatures.get(&100).unwrap();
        assert!(!creature.path.is_empty());
    }

    #[test]
    fn test_lua_set_path_to_solid() {
        let (lua, _gs) = setup_test_lua();
        // Set path to a solid tile (0,0)
        let result: bool = lua.load("return set_path(100, 0, 0)").eval().unwrap();
        assert!(!result);
    }

    #[test]
    fn test_lua_print() {
        let (lua, gs) = setup_test_lua();
        lua.load(r#"print("hello", "world", 42)"#).exec().unwrap();

        let gs_inner = gs.borrow();
        let output = gs_inner.print_output.borrow();
        assert_eq!(output.len(), 1);
        assert_eq!(output[0], "hello\tworld\t42");
    }

    #[test]
    fn test_lua_player_functions() {
        let (lua, _gs) = setup_test_lua();

        let exists: bool = lua.load("return player_exists(1)").eval().unwrap();
        assert!(exists);

        let exists: bool = lua.load("return player_exists(99)").eval().unwrap();
        assert!(!exists);

        let score: i32 = lua.load("return player_score(1)").eval().unwrap();
        assert_eq!(score, 42);

        let king = lua.load("return king_player()").eval::<Value>().unwrap();
        assert!(matches!(king, Value::Nil));

        let cpu: i32 = lua.load("return get_cpu_usage()").eval().unwrap();
        assert_eq!(cpu, 0);
    }

    #[test]
    fn test_lua_ownership_check() {
        let (lua, gs) = setup_test_lua();
        // Add a creature belonging to another player
        {
            let gs_inner = gs.borrow();
            let mut creatures = gs_inner.creatures.borrow_mut();
            creatures.insert(200, Creature::new(200, 768, 768, CREATURE_BIG, 2));
        }

        // Player 1 should not be able to set_path on creature 200
        let result = lua.load("return set_path(200, 512, 512)").eval::<bool>();
        assert!(result.is_err());

        // But get_pos should work (read-only, no ownership needed)
        let (x, y): (i32, i32) = lua.load("return get_pos(200)").eval().unwrap();
        assert_eq!(x, 768);
        assert_eq!(y, 768);
    }
}
