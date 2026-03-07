// LLM-friendly documentation endpoint content.

pub const LLMS_TXT: &str = r#"# SwarmCrest API
> A competitive bot programming platform where players write Lua bots to control creature swarms.

## API Base URL
/api/

## Authentication
API keys via Authorization header: `Authorization: Bearer infon_<key>`
Create keys in the web UI at /api-keys or via POST /api/api-keys.

## Key Endpoints
- GET/POST /api/bots - List/create bots
- GET/PUT/DELETE /api/bots/{id} - Get/update/delete bot
- GET/POST /api/bots/{id}/versions - List/create bot versions
- PUT /api/bots/{id}/active-version - Set active version
- GET /api/bots/{id}/stats - Get bot version stats
- GET /api/matches - List recent matches
- GET /api/matches/mine - User's own match history (auth required)
- GET /api/matches/{id} - Get match details
- GET /api/matches/{id}/replay - Get match replay data
- POST /api/matches/challenge - Create a challenge match
- POST /api/game/start - Start a live game
- GET /api/game/status - Check game status
- POST /api/game/stop - Stop current game
- GET /api/games/active - List active games
- GET /api/queue/status - Match queue status
- GET/POST /api/tournaments - List/create tournaments
- POST /api/tournaments/{id}/run - Run a tournament
- GET /api/leaderboards/1v1 - View 1v1 rankings
- GET /api/leaderboards/ffa - View FFA rankings
- GET /api/leaderboards/2v2 - View 2v2 rankings
- GET/POST /api/teams - List/create teams
- GET/POST /api/api-keys - List/create API keys
- DELETE /api/api-keys/{id} - Revoke API key
- POST /api/validate-lua - Validate Lua syntax
- GET /api/docs/lua-api - Lua API reference (Markdown)
- GET /api/maps - List available maps

## API Key Scopes
Create an API key via POST /api/api-keys with scopes like "bots:read,matches:write".
Available scopes: bots:read, bots:write, matches:read, matches:write, teams:write, api_keys:write, leaderboard:read
Default: bots:read,matches:read,leaderboard:read

## Bot Programming
Bots are written in Lua 5.1. The low-level API exposes C functions directly (set_path, get_pos, etc.).
Two high-level API styles wrap the low-level API (auto-detected from your code):
- Coroutine style (oo.lua): Define `Creature:main()` with blocking methods
- State machine style (state.lua): Define `bot()` with state functions and event handlers

See /api/docs/lua-api for the full API reference.

## Documentation Links
- Web docs (game mechanics, strategy, API auth guide): /docs
- Lua API reference (Markdown): /api/docs/lua-api
- Full LLM documentation: /llms-full.txt

## WebSocket
- /ws/game - Live game state stream (JSON frames)
"#;

pub const LLMS_FULL_TXT: &str = r#"# SwarmCrest - Complete Documentation

> SwarmCrest is a competitive bot programming platform where players write Lua scripts
> to control swarms of creatures competing for food and territory on a 2D tile-based map.
> Based on the original Infon Battle Arena by Florian Wesch, this web version brings the classic gameplay to the browser.

---

## Platform Overview

SwarmCrest is an open-source (GPL) multiplayer programming game. Players write Lua 5.1
scripts that control autonomous creatures in a shared game world. The game runs in 100ms ticks,
and each tick every creature's Lua coroutine is resumed to make decisions.

### Key Concepts
- **Creatures**: Autonomous units controlled by your Lua code
- **Food**: Resource on map tiles that creatures eat to survive and grow
- **King of the Hill**: A special tile; idle creatures on it score points for their player
- **Types**: 3 creature types with different stats (Small, Big, Flyer)
- **Spawning**: Big creatures (Type 1) can spawn new Small creatures (Type 0)
- **Converting**: Creatures can change type by spending food

---

## REST API Reference

### Authentication

All authenticated endpoints require an API key in the Authorization header:
```
Authorization: Bearer infon_<key>
```

API keys are created in the web UI at /api-keys or programmatically via POST /api/api-keys.
For more details on authentication setup, see the web documentation at /docs (REST API & Auth section).

### API Keys

API keys provide long-lived programmatic access. They do not expire but can be revoked.

**Create API Key:**
```
POST /api/api-keys
Authorization: Bearer <token>
Content-Type: application/json
{"name": "CI Key", "scopes": "bots:read,matches:read,matches:write"}
Response: {"id": 1, "name": "CI Key", "token": "infon_a1b2c3...", ...}
```
The `token` field is only returned on creation. Store it securely.

**Available Scopes:**
- bots:read - List and view bots and versions
- bots:write - Create/update/delete bots and versions
- matches:read - View matches, replays, and leaderboards
- matches:write - Create challenges and start games
- teams:write - Create/manage teams
- api_keys:write - Create new API keys
- leaderboard:read - View leaderboard rankings

Default scopes (if not specified): bots:read,matches:read,leaderboard:read

**List API Keys:**
```
GET /api/api-keys
Authorization: Bearer <token>
```

**Revoke API Key:**
```
DELETE /api/api-keys/{id}
Authorization: Bearer <token>
```

### Bots

**List Bots:**
```
GET /api/bots
GET /api/bots?all=true  (list all public bots)
Authorization: Bearer <token>
```

**Create Bot:**
```
POST /api/bots
Authorization: Bearer <token>
Content-Type: application/json
{"name": "MyBot", "description": "A simple bot"}
```

**Get/Update/Delete Bot:**
```
GET /api/bots/{id}
PUT /api/bots/{id}  {"name": "NewName", "description": "Updated"}
DELETE /api/bots/{id}
```

### Bot Versions

**List Versions:**
```
GET /api/bots/{id}/versions
```

**Create Version:**
```
POST /api/bots/{id}/versions
Content-Type: application/json
{"code": "function Creature:main()\n  self:eat()\nend"}
```

**Set Active Version:**
```
PUT /api/bots/{id}/active-version
Content-Type: application/json
{"version_id": 3}
```

**Get Bot Stats:**
```
GET /api/bots/{id}/stats
Response: [{version_id, elo_1v1, games_played, wins, losses, ...}]
```

### Matches

**List Recent Matches:**
```
GET /api/matches?limit=20
```

**List Your Matches:**
```
GET /api/matches/mine?limit=50&offset=0
Authorization: Bearer <token>
```

**Get Match Detail:**
```
GET /api/matches/{id}
Response: {"match": {...}, "participants": [...]}
```

**Get Match Replay:**
```
GET /api/matches/{id}/replay
Response: {"match_id", "tick_count", "messages": [...]}
```

**Create Challenge:**
```
POST /api/matches/challenge
Authorization: Bearer <token>
Content-Type: application/json
{
  "bot_version_id": 1,
  "opponent_bot_version_id": 2,
  "format": "1v1",
  "headless": true,
  "map": "default"
}
```

### Game Control (Live Games)

**Start Game:**
```
POST /api/game/start
Authorization: Bearer <token>
Content-Type: application/json
{
  "players": [{"bot_version_id": 1, "name": "Bot A"}, {"bot_version_id": 2}],
  "map": "random"
}
```

**Game Status:**
```
GET /api/game/status
Response: {"running": true}
```

**Stop Game:**
```
POST /api/game/stop
Authorization: Bearer <token>
```

**Active Games:**
```
GET /api/games/active
Response: [{match_id, player_names, format, map, spectator_count, ...}]
```

**Queue Status:**
```
GET /api/queue/status
```

### Tournaments

**List/Create Tournaments:**
```
GET /api/tournaments
POST /api/tournaments  {"name": "Weekly", "map": "default"}
```

**Get Tournament:**
```
GET /api/tournaments/{id}
```

**Update Tournament:**
```
PUT /api/tournaments/{id}
{"format": "round_robin", "config": "{}"}
Formats: round_robin, single_elimination, swiss_N
```

**Tournament Entries:**
```
GET /api/tournaments/{id}/entries
POST /api/tournaments/{id}/entries  {"bot_version_id": 1}
DELETE /api/tournaments/{id}/entries/{entry_id}
```

**Run Tournament:**
```
POST /api/tournaments/{id}/run
```

**Standings & Results:**
```
GET /api/tournaments/{id}/standings
GET /api/tournaments/{id}/results
```

### Leaderboards

```
GET /api/leaderboards/1v1?limit=50&offset=0
GET /api/leaderboards/ffa?limit=50&offset=0
GET /api/leaderboards/2v2?limit=50&offset=0
```

### Teams (2v2)

```
GET /api/teams
POST /api/teams  {"name": "Dream Team"}
GET /api/teams/{id}
PUT /api/teams/{id}  {"name": "New Name"}
DELETE /api/teams/{id}
GET /api/teams/{id}/versions
POST /api/teams/{id}/versions  {"bot_version_a": 1, "bot_version_b": 2}
```

### Maps

```
GET /api/maps
Response: [{"name": "random", "width": 30, "height": 30, "description": "..."}]
```

### Lua Validation

```
POST /api/validate-lua
Authorization: Bearer <token>
Content-Type: application/json
{"code": "function Creature:main() end"}
Response: {"valid": true} or {"valid": false, "error": "..."}
```

### Documentation & Further Reading

```
GET /api/docs/lua-api  - Lua API reference (Markdown)
GET /llms.txt          - LLM-friendly summary
GET /llms-full.txt     - Complete LLM documentation (this file)
```

**Web documentation** at /docs covers:
- Getting Started guide for new players
- Lua API reference with examples
- Strategy Guide with creature stats, combat math, economy details
- REST API & Auth guide with API key setup walkthrough

### WebSocket

```
WS /ws/game  - Live game state stream
```

Messages:
- `world`: Map dimensions, tiles, KotH position
- `snapshot`: Creature positions, player scores (sent each tick)
- `snapshot_delta`: Incremental creature updates (changed/removed)
- `game_end`: Final scores, winner, match ID, player stats
- `player_load_error`: Lua loading errors

---

## Game Mechanics

### Game World
- 2D tile-based grid (each tile 256x256 units/pixels)
- X increases rightward, Y increases downward
- Tiles: TILE_SOLID (0, walls) or TILE_PLAIN (1, walkable)
- Game runs in 100ms ticks (10 ticks per second)
- world_size() returns playable boundaries as x1, y1, x2, y2

### Creature Type Stats

| Stat | Small (Type 0) | Big (Type 1) | Flyer (Type 2) |
|------|---------------|-------------|----------------|
| Max Health | 10,000 | 20,000 | 5,000 |
| Max Food | 10,000 | 20,000 | 5,000 |
| Base Speed | 200 px/s | 400 px/s | 800 px/s |
| Speed Bonus | +625 × health/max_health | None | None |
| Health Drain | 50/s (5/tick) | 70/s (7/tick) | 50/s (5/tick) |
| Heal Rate | 500 HP/s (from food) | 300 HP/s | 600 HP/s |
| Eat Rate | 800 food/s (from tile) | 400 food/s | 600 food/s |
| Can Spawn | No | Yes (Type 0 offspring) | No |
| Can Feed | Yes (256px, 400 food/s) | No | Yes (256px, 400 food/s) |
| Flies Over Walls | No | No | Yes |

Small creatures have a unique speed bonus: effective speed = 200 + 625 × (current_health / max_health).
At full health a Small moves at 825 px/s, nearly as fast as a Flyer.

### Combat System

Combat is continuous DPS. While in ATTACK state with target in range, damage is applied every tick:
`damage = damage_per_sec × tick_delta_ms / 1000`
Range is Euclidean distance in pixels. If target exits range or dies, attacker goes IDLE.

**Damage Table (DPS / Range in pixels):**

| Attacker | vs Small | vs Big | vs Flyer |
|----------|----------|--------|----------|
| Small | 0 / -- | 0 / -- | 1,000 DPS / 768px |
| Big | 1,500 DPS / 512px | 1,500 DPS / 512px | 1,500 DPS / 512px |
| Flyer | Cannot attack | Cannot attack | Cannot attack |

Time-to-kill examples: Big vs Big (20,000 HP) = ~13.3s. Big vs Small (10,000 HP) = ~6.7s.
Small vs Flyer (5,000 HP) = 5s.

### Conversion Costs

Food is consumed at 1,000 food/s during conversion:
- Small → Big: 8,000 food (8s)
- Small → Flyer: 5,000 food (5s)
- Big → Small: 8,000 food (8s)
- Flyer → Small: 5,000 food (5s)
- Big ↔ Flyer: Not allowed (must go through Small)

### Spawning (Big Only)

- Food cost: 5,000 (consumed at 2,000 food/s)
- Health cost: 4,000 HP (deducted immediately at spawn start)
- Offspring type: Small (Type 0)

### Food & Tile Economy

- Max food per tile: 9,999
- Initial food at each spawner: 9,000
- Food spawners are map-defined points with radius, amount, and interval
- Respawn: periodic (typically every 3,000-5,000ms), placing food on a random tile within spawner radius
- Respawn amount: map-dependent (typically ~800 food per event for generated maps)
- Food does NOT grow continuously; it appears in discrete chunks at interval boundaries

### CPU Limits

- Each player limited to 500,000 Lua VM instructions per tick
- If exceeded: current tick's player_think() aborts, error logged to console output
- Creatures are NOT killed. Bot is NOT kicked. Game continues normally next tick
- get_cpu_usage() currently returns 0 (stub — no real-time tracking)

### Creature States
- CREATURE_IDLE (0): Doing nothing; required for King of the Hill scoring
- CREATURE_WALK (1): Moving to destination
- CREATURE_HEAL (2): Converting food to health (type-dependent rate)
- CREATURE_EAT (3): Consuming food from tile (type-dependent rate)
- CREATURE_ATTACK (4): Continuous DPS to target while in range
- CREATURE_CONVERT (5): Changing type (consumes food at 1,000/s)
- CREATURE_SPAWN (6): Type 1 producing Type 0 (consumes food at 2,000/s)
- CREATURE_FEED (7): Transferring food to ally (Small/Flyer only, 256px range, 400 food/s)

### King of the Hill
- Special tile on the map at a fixed position
- Creature in IDLE state on tile becomes king
- King's player scores points each tick
- king_player() returns current king's player ID

### Scoring
- Points from holding King of the Hill
- suicide() costs 40 points
- Players may be kicked if score drops too low

---

## Lua API Reference

### Constants
```lua
CREATURE_IDLE=0  CREATURE_WALK=1  CREATURE_HEAL=2  CREATURE_EAT=3
CREATURE_ATTACK=4  CREATURE_CONVERT=5  CREATURE_SPAWN=6  CREATURE_FEED=7
TILE_SOLID=0  TILE_PLAIN=1
CREATURE_SPAWNED=0  CREATURE_KILLED=1  CREATURE_ATTACKED=2  PLAYER_CREATED=3
```

### Low-Level API

**Creature Actions:**
- set_path(id, x, y) -> bool: Set movement destination
- set_state(id, state) -> bool: Set creature state
- get_state(id) -> state: Get current state
- set_target(id, target_id) -> bool: Set attack/feed target
- set_convert(id, type) -> bool: Set conversion type (0,1,2)
- suicide(id): Kill creature, drop 1/3 food
- set_message(id, msg): Display message (max 8 chars)

**Creature Queries:**
- get_pos(id) -> x, y
- get_type(id) -> type
- get_food(id) -> food (own only)
- get_health(id) -> 0-100
- get_speed(id) -> speed (own only)
- get_tile_food(id) -> food (own only)
- get_tile_type(id) -> type
- get_max_food(id) -> food (own only)
- get_distance(id, target_id) -> dist
- get_nearest_enemy(id) -> id, x, y, playernum, dist (or nil)
- creature_exists(id) -> bool
- creature_player(id) -> player_no

**World Functions:**
- world_size() -> x1, y1, x2, y2
- game_time() -> ms
- get_koth_pos() -> x, y
- player_exists(id) -> bool
- king_player() -> player_id
- player_score(id) -> score
- get_cpu_usage() -> 0-100
- print(msg)

### High-Level API: Coroutine Style (oo.lua)

Entry point: `function Creature:main() ... end`

**Blocking methods:** moveto(x,y), heal(), eat(), feed(target), attack(target), convert(type), spawn(), suicide()

**Non-blocking:** begin_idling(), begin_walk_path(), begin_healing(), begin_eating(), begin_attacking(), begin_converting(), begin_spawning(), begin_feeding()

**Properties:** self.id, self:pos(), self:speed(), self:health(), self:food(), self:max_food(), self:tile_food(), self:tile_type(), self:type(), self:distance(other), self:nearest_enemy()

**Utility:** set_path(x,y), set_target(c), set_conversion(t), screen_message(msg), sleep(ms), wait_for_next_round(), restart()

**Callbacks:** Creature:onSpawned(parent_id), Creature:onKilled(killer_id), Creature:onAttacked(attacker_id)

### High-Level API: State Machine Style (state.lua)

Entry point: `function bot() ... end`

**State transitions:** and_start_state(name,...), and_be_in_state(name,...), and_keep_state, and_restart_state, in_state(name)

**Actions:** move_to(x,y), move_path(x,y), random_move(), random_path(), heal(), eat(), feed(target), attack(target), convert(type), spawn(), sleep(ms)

**Properties:** food(), health(), max_food(), tile_food(), tile_type(), type(), speed(), pos(), can_eat()

**Events:** onSpawned(parent_id), onKilled(killer_id), onIdle(), onTileFood(), onLowHealth(), onTick()

### Example Bot (Coroutine Style)
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
"#;
