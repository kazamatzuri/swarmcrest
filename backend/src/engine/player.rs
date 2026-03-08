use mlua::Lua;

use super::config::LUA_MAX_INSTRUCTIONS;
use super::lua_api;

/// Which high-level API style the bot uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiStyle {
    /// Coroutine-based: `Creature:main()`, blocking methods like `self:moveto()`.
    Oo,
    /// State-machine-based: `bot()` with state functions and event handlers.
    State,
}

/// Represents a player controlling a swarm of creatures.
pub struct Player {
    pub id: u32,
    pub name: String,
    pub score: i32,
    pub color: u8,
    pub num_creatures: i32,
    pub lua: Lua,
    pub output: Vec<String>,
}

impl Player {
    /// Create a new player with a fresh Lua VM.
    /// Registers all API functions, constants, and bootstrap code.
    /// The high-level API (oo.lua or state.lua) is NOT loaded here — it is
    /// auto-detected and loaded in `load_code()` based on the bot source.
    pub fn new(id: u32, name: &str) -> Result<Self, String> {
        // SAFETY: We need the debug library for debug.sethook to set instruction
        // limits on coroutine threads. The debug global is removed in the bootstrap
        // after saving a reference to debug.sethook, so user code cannot access it.
        let lua = unsafe { Lua::unsafe_new() };

        // Set memory limit to 64MB to prevent runaway scripts
        lua.set_memory_limit(64 * 1024 * 1024)
            .map_err(|e| format!("Failed to set memory limit: {e}"))?;

        // Register constants and API functions
        lua_api::register_constants(&lua, id)
            .map_err(|e| format!("Failed to register constants: {e}"))?;
        lua_api::register_functions(&lua, id)
            .map_err(|e| format!("Failed to register API functions: {e}"))?;

        // Provide _TRACEBACK as a simple passthrough (debug.traceback removed in sandbox)
        lua.load(
            r#"
            function _TRACEBACK(...)
                return tostring((...))
            end
            function epcall(handler, func, ...)
                return xpcall(func, handler, ...)
            end
            "#,
        )
        .exec()
        .map_err(|e| format!("Failed to set up _TRACEBACK/epcall: {e}"))?;

        // Compatibility aliases and bootstrap (from original player.lua)
        let bootstrap = format!(
            r#"
-- Compatibility aliases (from player.lua)
nearest_enemy = get_nearest_enemy
exists = creature_exists

-- creature_config metatable
creature_config = setmetatable({{}}, {{
    __index = function(t, val)
        return creature_get_config(val)
    end
}})

-- Empty Creature table for OO-style bots to define methods on.
-- The full Creature class methods are loaded later by oo.lua.
Creature = {{}}

-- needs_api: accepted for backward compatibility but API loading
-- is handled automatically by the engine. Both "oo" and "state" are valid.
function needs_api(needed)
    assert(needed == "oo" or needed == "state",
           "Unknown API style '" .. tostring(needed) .. "'. Use 'oo' or 'state'.")
end

-- Switch print to client_print
print = client_print

-- p() pretty-print helper
function p(x)
    if type(x) == "table" then
        print("+--- Table: " .. tostring(x))
        for key, val in pairs(x) do
            print("| " .. tostring(key) .. " " .. tostring(val))
        end
        print("+-----------------------")
    else
        print(type(x) .. " - " .. tostring(x))
    end
end

-- restart and info functions used by oo.lua
function restart()
    for id, creature in pairs(creatures) do
        creature:restart()
    end
end

function info()
    for id, creature in pairs(creatures) do
        print(tostring(creature))
    end
end

-- Default onCommand
function onCommand(cmd)
    print("huh? use '?' for help")
end

-- Instruction limit for coroutines: Lua 5.1 hooks are per-thread,
-- so we wrap coroutine.resume to install the hook on each coroutine.
do
    local _sethook = debug.sethook
    local _resume = coroutine.resume
    local _instruction_limit = {LUA_MAX_INSTRUCTIONS}
    coroutine.resume = function(co, ...)
        _sethook(co, function() error("lua vm cycles exceeded") end, "", _instruction_limit)
        local results = {{_resume(co, ...)}}
        _sethook(co)
        -- If resume failed with cycles exceeded, print so it appears in output
        if not results[1] and type(results[2]) == "string" and results[2]:find("cycles exceeded") then
            print("Lua error: " .. results[2])
        end
        return unpack(results)
    end
end

-- Disable dangerous functions for sandbox
debug = nil
load = nil
require = nil
loadfile = nil
os = nil
package = nil
io = nil
module = nil
collectgarbage = nil
"#
        );
        lua.load(&bootstrap)
            .set_name("bootstrap")
            .exec()
            .map_err(|e| format!("Failed to load bootstrap: {e}"))?;

        Ok(Player {
            id,
            name: name.to_string(),
            score: 0,
            color: (id % 16) as u8,
            num_creatures: 0,
            lua,
            output: Vec::new(),
        })
    }

    /// Detect which high-level API style the bot source uses.
    ///
    /// Detection rules (checked against the source text):
    /// 1. Explicit `needs_api("state")` or `needs_api('state')` → State
    /// 2. `function bot()` defined → State
    /// 3. Everything else (including `needs_api("oo")`) → OO (default)
    fn detect_api_style(code: &str) -> ApiStyle {
        // Check for explicit needs_api("state") / needs_api('state')
        if code.contains("needs_api(\"state\")")
            || code.contains("needs_api('state')")
            || code.contains("needs_api \"state\"")
            || code.contains("needs_api 'state'")
        {
            return ApiStyle::State;
        }

        // Check for function bot() pattern (state-machine style entry point)
        if code.contains("function bot()") || code.contains("function bot ()") {
            return ApiStyle::State;
        }

        ApiStyle::Oo
    }

    /// Load the appropriate high-level API files into the Lua VM.
    fn load_api(&self, style: ApiStyle) -> Result<(), String> {
        match style {
            ApiStyle::Oo => {
                let api_code = include_str!("../../../orig_game/api/oo.lua");
                self.lua
                    .load(api_code)
                    .set_name("api/oo.lua")
                    .exec()
                    .map_err(|e| format!("Failed to load high-level API (oo.lua): {e}"))?;

                let default_code = include_str!("../../../orig_game/api/oo-default.lua");
                self.lua
                    .load(default_code)
                    .set_name("api/oo-default.lua")
                    .exec()
                    .map_err(|e| {
                        format!("Failed to load high-level API defaults (oo-default.lua): {e}")
                    })?;
            }
            ApiStyle::State => {
                let api_code = include_str!("../../../orig_game/api/state.lua");
                self.lua
                    .load(api_code)
                    .set_name("api/state.lua")
                    .exec()
                    .map_err(|e| format!("Failed to load high-level API (state.lua): {e}"))?;

                let default_code = include_str!("../../../orig_game/api/state-default.lua");
                self.lua
                    .load(default_code)
                    .set_name("api/state-default.lua")
                    .exec()
                    .map_err(|e| {
                        format!("Failed to load high-level API defaults (state-default.lua): {e}")
                    })?;
            }
        }
        Ok(())
    }

    /// Load user bot code into the Lua VM.
    /// Auto-detects whether the bot uses the OO or State high-level API style,
    /// loads the appropriate API wrapper, then executes the user's bot code.
    /// Game state must be set in app_data before calling this, so top-level
    /// bot code (e.g. `world_size()` calls) can access the game world.
    pub fn load_code(&self, code: &str) -> Result<(), String> {
        let style = Self::detect_api_style(code);

        // Load the high-level API + defaults BEFORE user code, so that
        // user code can override default callbacks (Creature:main, bot(), etc.)
        self.load_api(style)?;

        if !code.is_empty() {
            self.lua
                .load(code)
                .set_name("user_bot")
                .exec()
                .map_err(|e| format!("Failed to load bot code: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_player() {
        let player = Player::new(1, "TestBot");
        assert!(player.is_ok());
        let player = player.unwrap();
        assert_eq!(player.id, 1);
        assert_eq!(player.name, "TestBot");
        assert_eq!(player.score, 0);
    }

    #[test]
    fn test_player_lua_constants() {
        let player = Player::new(1, "TestBot").unwrap();
        let lua = &player.lua;

        // Check creature type constants
        let val: i32 = lua.globals().get("CREATURE_SMALL").unwrap();
        assert_eq!(val, 0);
        let val: i32 = lua.globals().get("CREATURE_BIG").unwrap();
        assert_eq!(val, 1);
        let val: i32 = lua.globals().get("CREATURE_FLYER").unwrap();
        assert_eq!(val, 2);

        // Check creature state constants
        let val: i32 = lua.globals().get("CREATURE_IDLE").unwrap();
        assert_eq!(val, 0);
        let val: i32 = lua.globals().get("CREATURE_WALK").unwrap();
        assert_eq!(val, 1);
        let val: i32 = lua.globals().get("CREATURE_ATTACK").unwrap();
        assert_eq!(val, 4);

        // Check event constants
        let val: i32 = lua.globals().get("CREATURE_SPAWNED").unwrap();
        assert_eq!(val, 0);
        let val: i32 = lua.globals().get("CREATURE_KILLED").unwrap();
        assert_eq!(val, 1);
        let val: i32 = lua.globals().get("CREATURE_ATTACKED").unwrap();
        assert_eq!(val, 2);
        let val: i32 = lua.globals().get("PLAYER_CREATED").unwrap();
        assert_eq!(val, 3);

        // Check tile constants
        let val: i32 = lua.globals().get("TILE_SOLID").unwrap();
        assert_eq!(val, 0);
        let val: i32 = lua.globals().get("TILE_PLAIN").unwrap();
        assert_eq!(val, 1);

        // Check player_number
        let val: u32 = lua.globals().get("player_number").unwrap();
        assert_eq!(val, 1);
    }

    #[test]
    fn test_player_think_exists_after_oo_load() {
        let player = Player::new(1, "TestBot").unwrap();
        // player_think is defined by the high-level API, loaded in load_code()
        player.load_code("function Creature:main() end").unwrap();
        let _func: mlua::Function = player.lua.globals().get("player_think").unwrap();
    }

    #[test]
    fn test_player_think_exists_after_state_load() {
        let player = Player::new(1, "TestBot").unwrap();
        player
            .load_code("function bot() function onIdle() end end")
            .unwrap();
        let _func: mlua::Function = player.lua.globals().get("player_think").unwrap();
    }

    #[test]
    fn test_detect_oo_style() {
        assert_eq!(
            Player::detect_api_style("function Creature:main() end"),
            ApiStyle::Oo
        );
        assert_eq!(
            Player::detect_api_style("needs_api(\"oo\")\nfunction Creature:main() end"),
            ApiStyle::Oo
        );
        assert_eq!(Player::detect_api_style(""), ApiStyle::Oo);
    }

    #[test]
    fn test_detect_state_style() {
        assert_eq!(
            Player::detect_api_style("function bot()\n  function onIdle() end\nend"),
            ApiStyle::State
        );
        assert_eq!(
            Player::detect_api_style("needs_api(\"state\")\nfunction bot() end"),
            ApiStyle::State
        );
        assert_eq!(
            Player::detect_api_style("needs_api('state')\nfunction bot() end"),
            ApiStyle::State
        );
        assert_eq!(
            Player::detect_api_style("needs_api \"state\"\nfunction bot() end"),
            ApiStyle::State
        );
    }

    #[test]
    fn test_needs_api_accepts_both_styles() {
        let player = Player::new(1, "TestBot").unwrap();
        // needs_api("oo") should not error
        player
            .load_code("needs_api(\"oo\")\nfunction Creature:main() end")
            .unwrap();

        let player2 = Player::new(2, "TestBot2").unwrap();
        // needs_api("state") should not error
        player2
            .load_code("needs_api(\"state\")\nfunction bot() function onIdle() end end")
            .unwrap();
    }
}
