// Integration tests: run original Infon bots in the game engine.
//
// These tests verify that the original bot Lua code loads and runs
// without errors in our Rust game engine implementation.

use swarmcrest_backend::engine::config::*;
use swarmcrest_backend::engine::game::Game;
use swarmcrest_backend::engine::world::World;

/// Create a 20x20 world with walkable interior and food.
fn create_test_world() -> World {
    let mut world = World::new(20, 20);
    // Make interior walkable
    for x in 1..19 {
        for y in 1..19 {
            world.set_type(x, y, TILE_PLAIN);
        }
    }
    // Set koth in center
    world.koth_x = 10;
    world.koth_y = 10;
    // Add food to many tiles
    for x in 3..17 {
        for y in 3..17 {
            world.add_food(x, y, 5000);
        }
    }
    world
}

// ---- Coroutine-style (oo.lua) bots ----

#[test]
fn test_stupibot_loads() {
    let world = create_test_world();
    let mut game = Game::new(world);

    let code = include_str!("test_bots/stupibot.lua");
    let result = game.add_player("stupibot", code);
    assert!(result.is_ok(), "Failed to add stupibot: {:?}", result.err());
}

#[test]
fn test_stupibot_runs_50_ticks() {
    let world = create_test_world();
    let mut game = Game::new(world);

    let code = include_str!("test_bots/stupibot.lua");
    let player_id = game
        .add_player("stupibot", code)
        .expect("Failed to add stupibot");

    // Spawn a creature on a food tile
    let spawn_x = World::tile_center(5);
    let spawn_y = World::tile_center(5);
    let creature_id = game
        .spawn_creature(player_id, spawn_x, spawn_y, CREATURE_SMALL)
        .expect("Failed to spawn creature");

    // Run 50 ticks
    for _ in 0..50 {
        game.tick();
    }

    // Creature should still be alive (has food to eat)
    let creatures = game.creatures.borrow();
    assert!(
        creatures.contains_key(&creature_id),
        "stupibot creature should still exist after 50 ticks"
    );
}

#[test]
fn test_stupibot_eats_food() {
    let world = create_test_world();
    let mut game = Game::new(world);

    let code = include_str!("test_bots/stupibot.lua");
    let player_id = game.add_player("stupibot", code).unwrap();

    let spawn_x = World::tile_center(5);
    let spawn_y = World::tile_center(5);
    let creature_id = game
        .spawn_creature(player_id, spawn_x, spawn_y, CREATURE_SMALL)
        .unwrap();

    // Run enough ticks for the bot to start eating
    for _ in 0..30 {
        game.tick();
    }

    let creatures = game.creatures.borrow();
    if let Some(creature) = creatures.get(&creature_id) {
        // stupibot's eatAndGrow should make small creatures eat when there's tile food
        assert!(
            creature.food > 0,
            "stupibot creature should have eaten food, but food is {}",
            creature.food
        );
    }
}

#[test]
fn test_stupibot2_loads_and_runs() {
    let world = create_test_world();
    let mut game = Game::new(world);

    let code = include_str!("test_bots/stupibot2.lua");
    let player_id = game
        .add_player("stupibot2", code)
        .expect("Failed to add stupibot2");

    let spawn_x = World::tile_center(5);
    let spawn_y = World::tile_center(5);
    game.spawn_creature(player_id, spawn_x, spawn_y, CREATURE_SMALL)
        .expect("Failed to spawn creature");

    for _ in 0..50 {
        game.tick();
    }
}

#[test]
fn test_sissy_bot_loads_and_runs() {
    let world = create_test_world();
    let mut game = Game::new(world);

    let code = include_str!("test_bots/sissy-bot.lua");
    let player_id = game
        .add_player("sissy-bot", code)
        .expect("Failed to add sissy-bot");

    let spawn_x = World::tile_center(5);
    let spawn_y = World::tile_center(5);
    game.spawn_creature(player_id, spawn_x, spawn_y, CREATURE_SMALL)
        .expect("Failed to spawn creature");

    // sissy-bot is complex; run many ticks
    for _ in 0..100 {
        game.tick();
    }
}

// ---- Multi-player test ----

#[test]
fn test_stupibot_vs_stupibot() {
    let world = create_test_world();
    let mut game = Game::new(world);

    let code = include_str!("test_bots/stupibot.lua");
    let p1 = game.add_player("stupibot1", code).unwrap();
    let p2 = game.add_player("stupibot2_player", code).unwrap();

    // Spawn creatures for both players
    game.spawn_creature(
        p1,
        World::tile_center(5),
        World::tile_center(5),
        CREATURE_SMALL,
    );
    game.spawn_creature(
        p2,
        World::tile_center(15),
        World::tile_center(15),
        CREATURE_SMALL,
    );

    // Run 200 ticks (20 seconds of game time)
    for _ in 0..200 {
        game.tick();
    }

    // Game should have run without panics
    assert!(game.game_time >= 20000);
}

// ---- No-error check: verify player output doesn't contain critical Lua errors ----

#[test]
fn test_stupibot_no_lua_errors() {
    let world = create_test_world();
    let mut game = Game::new(world);

    let code = include_str!("test_bots/stupibot.lua");
    let player_id = game.add_player("stupibot", code).unwrap();

    game.spawn_creature(
        player_id,
        World::tile_center(5),
        World::tile_center(5),
        CREATURE_SMALL,
    );

    for _ in 0..30 {
        game.tick();
    }

    let player = game.players.get(&player_id).unwrap();
    let errors: Vec<&String> = player
        .output
        .iter()
        .filter(|s| s.starts_with("Lua error"))
        .collect();
    assert!(
        errors.is_empty(),
        "stupibot produced Lua errors: {:?}",
        errors
    );
}
