# SwarmCrest Lua API Reference

This documents the complete Lua API available to bot programmers. There are two API layers:
1. **Low-level API** - Direct C functions exposed to Lua
2. **High-level APIs** - Lua wrappers (`oo.lua` for object-oriented, `state.lua` for state machines)

## Constants

### Creature States
```lua
CREATURE_IDLE    = 0  -- Not doing anything
CREATURE_WALK    = 1  -- Moving
CREATURE_HEAL    = 2  -- Recovering health
CREATURE_EAT     = 3  -- Eating food from tile
CREATURE_ATTACK  = 4  -- Attacking target
CREATURE_CONVERT = 5  -- Converting to new type
CREATURE_SPAWN   = 6  -- Spawning offspring
CREATURE_FEED    = 7  -- Feeding another creature
```

### Tile Types
```lua
TILE_SOLID = 0  -- Wall, unwalkable
TILE_PLAIN = 1  -- Walkable ground
```

### Event Types
```lua
CREATURE_SPAWNED  = 0  -- Creature was created
CREATURE_KILLED   = 1  -- Creature died
CREATURE_ATTACKED = 2  -- Creature was attacked
PLAYER_CREATED    = 3  -- New player joined
```

### Predefined Variables
```lua
MAXPLAYER       -- Maximum number of players
player_number   -- Your player ID
```

---

## Low-Level API

### Creature Actions

| Function | Description |
|----------|-------------|
| `set_path(id, x, y) -> bool` | Set movement destination (pathfinding). |
| `set_state(id, state) -> bool` | Set creature state constant. |
| `get_state(id) -> state` | Get current state. |
| `set_target(id, target_id) -> bool` | Set attack/feed target. |
| `set_convert(id, type) -> bool` | Set conversion target type (0, 1, or 2). |
| `suicide(id)` | Kill creature. 1/3 food placed on tile. Costs 40 points. |
| `set_message(id, msg)` | Display message below creature (max 8 chars). |

### Creature Queries

| Function | Description |
|----------|-------------|
| `get_pos(id) -> x, y` | Get creature coordinates. |
| `get_type(id) -> type` | Get creature type (0, 1, or 2). |
| `get_food(id) -> food` | Get stored food (own creatures only). |
| `get_health(id) -> 0-100` | Get health percentage. |
| `get_speed(id) -> speed` | Get movement speed (own creatures only). |
| `get_tile_food(id) -> food` | Get food on creature's current tile (own only). |
| `get_tile_type(id) -> type` | Get tile type at creature's position. |
| `get_max_food(id) -> food` | Get max food capacity (own only). |
| `get_distance(id, target_id) -> dist` | Distance between two creatures. |
| `get_nearest_enemy(id) -> id, x, y, playernum, dist` | Info about nearest enemy (or nil). |
| `creature_exists(id) -> bool` | Check if creature exists. |
| `creature_player(id) -> player_no` | Get creature's player number. |

### World / Game Functions

| Function | Description |
|----------|-------------|
| `world_size() -> x1, y1, x2, y2` | Get world boundaries. |
| `game_time() -> ms` | Milliseconds since game start. |
| `get_koth_pos() -> x, y` | King of the Hill tile center. |
| `player_exists(id) -> bool` | Check if player exists. |
| `king_player() -> player_id` | Current king's player. |
| `player_score(id) -> score` | Get a player's score. |
| `get_cpu_usage() -> 0-100` | Your CPU usage. |

### Communication

| Function | Description |
|----------|-------------|
| `print(msg)` | Print message on all player connections. |

---

## High-Level API: Coroutine Style (oo.lua)

This high-level API wraps low-level functions into a `Creature` class. Each creature runs `Creature:main()` as a coroutine. This is the default style if no `needs_api()` call is present.

### Entry Point

```lua
function Creature:main()
    -- Your creature logic here
    -- Runs as a coroutine, can use blocking methods
end
```

### Blocking Methods (wait for completion)

| Method | Description |
|--------|-------------|
| `self:moveto(x, y) -> bool` | Move to coordinates; blocks until arrival. |
| `self:heal() -> bool` | Heal until full or out of food. |
| `self:eat() -> bool` | Eat until full or tile empty. |
| `self:feed(target) -> bool` | Feed target creature. |
| `self:attack(target) -> bool` | Attack target until dead or out of range. |
| `self:convert(type) -> bool` | Convert to specified type. |
| `self:spawn() -> bool` | Spawn a new creature. |
| `self:suicide()` | Kill this creature. |

### Non-Blocking Methods (begin_ prefix)

| Method | Description |
|--------|-------------|
| `self:begin_idling()` | Set to CREATURE_IDLE. |
| `self:begin_walk_path()` | Set to CREATURE_WALK. |
| `self:begin_healing()` | Set to CREATURE_HEAL. |
| `self:begin_eating()` | Set to CREATURE_EAT. |
| `self:begin_attacking()` | Set to CREATURE_ATTACK. |
| `self:begin_converting()` | Set to CREATURE_CONVERT. |
| `self:begin_spawning()` | Set to CREATURE_SPAWN. |
| `self:begin_feeding()` | Set to CREATURE_FEED. |

### State Checks

`self:is_idle()`, `is_walking()`, `is_healing()`, `is_eating()`, `is_attacking()`, `is_converting()`, `is_spawning()`, `is_feeding()` -- each returns boolean.

### Property Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `self.id` | number | Unique creature ID |
| `self:pos()` | x, y | Current position |
| `self:speed()` | number | Movement speed |
| `self:health()` | 0-100 | Health percentage |
| `self:food()` | number | Stored food |
| `self:max_food()` | number | Max food capacity |
| `self:tile_food()` | number | Food on current tile |
| `self:tile_type()` | number | Current tile type |
| `self:type()` | 0-2 | Creature type |
| `self:distance(other)` | number | Distance to another creature |
| `self:nearest_enemy()` | id,x,y,pnum,dist | Nearest enemy info |

### Utility

| Method | Description |
|--------|-------------|
| `self:set_path(x, y)` | Set movement destination. |
| `self:set_target(c)` | Set attack/feed target. |
| `self:set_conversion(t)` | Set conversion target type. |
| `self:screen_message(msg)` | Display message below creature. |
| `self:sleep(msec)` | Pause execution. |
| `self:wait_for_next_round()` | Yield until next game round. |
| `self:restart()` | Reset and restart coroutine. |

### Callbacks

| Callback | Description |
|----------|-------------|
| `Creature:onSpawned(parent_id)` | Called when creature is created. Can use blocking ops. |
| `Creature:onKilled(killer_id)` | Called after death. Cannot call creature API. |
| `Creature:onAttacked(attacker_id)` | Called when hit. Cannot use blocking ops. |

---

## High-Level API: State Machine Style (state.lua)

This high-level API uses a state-machine pattern where the bot function defines states as nested functions. Detected automatically when your code defines `function bot()` or calls `needs_api("state")`.

### Entry Point

```lua
function bot()
    -- Define states and event handlers here

    function find_food()
        random_move()
    end

    function eat_food()
        eat()
        return and_start_state "find_food"
    end

    function onIdle()
        return and_start_state "find_food"
    end
end
```

### State Transitions

| Function | Description |
|----------|-------------|
| `and_start_state(name, ...)` | Force transition to named state. |
| `and_be_in_state(name, ...)` | Transition if not already in state; returns true if already there. |
| `and_keep_state` (= false) | Stay in current state. |
| `and_restart_state` (= true) | Restart current state. |
| `in_state(name) -> bool` | Check if currently in state. |

### Action Functions (Blocking)

`move_to(x,y)`, `move_path(x,y)`, `random_move()`, `random_path()`, `heal()`, `eat()`, `feed(target)`, `attack(target)`, `convert(type)`, `spawn()`, `sleep(ms)`

### Property Functions

`food()`, `health()`, `max_food()`, `tile_food()`, `tile_type()`, `type()`, `speed()`, `pos()`, `can_eat()`

### Event Handlers

| Handler | Description |
|---------|-------------|
| `onSpawned(parent_id)` | Creature created. |
| `onKilled(killer_id)` | Creature died. |
| `onIdle()` | Creature became idle. |
| `onTileFood()` | Creature is on tile with food. |
| `onLowHealth()` | Health below 5. |
| `onTick()` | Called each round. |

---

## Example Bots

### Minimal (Coroutine Style)
```lua
function Creature:main()
    self:screen_message("Hi!")
end
```

### Random Walker (Coroutine Style)
```lua
function Creature:main()
    while true do
        local x1, y1, x2, y2 = world_size()
        self:moveto(math.random(x1, x2), math.random(y1, y2))
    end
end
```

### Eat and Grow (Coroutine Style)
```lua
function Creature:main()
    while true do
        if self:health() < 50 then
            self:heal()
        elseif self:tile_food() > 0 and self:food() < self:max_food() then
            self:eat()
        elseif self:type() == 0 and self:food() > 8000 then
            self:convert(1)
        elseif self:type() == 1 and self:food() > 8000 and self:health() > 60 then
            self:spawn()
        else
            local x1, y1, x2, y2 = world_size()
            self:moveto(math.random(x1, x2), math.random(y1, y2))
        end
    end
end
```

### Simple State Bot (State Machine Style)
```lua
function bot()
    function onIdle()
        return and_start_state "find_food"
    end

    function find_food()
        random_move()
    end

    function onTileFood()
        if can_eat() then
            return and_be_in_state "eat_food"
        end
    end

    function eat_food()
        eat()
        return and_start_state "find_food"
    end
end
```
