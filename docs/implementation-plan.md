# Web-Based SwarmCrest - Implementation Plan

## Vision

Build a web-based version of SwarmCrest where players write Lua bots in a browser-based editor, manage a bot library, and run tournaments. The MVP focuses on a single-user experience with no authentication.

## Architecture Overview

```
┌─────────────────────────────────────────────────┐
│                   Browser                        │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐ │
│  │ Lua Code │  │  Bot      │  │  Game Canvas  │ │
│  │ Editor   │  │  Library  │  │  (Renderer)   │ │
│  │ (Monaco) │  │  Manager  │  │               │ │
│  └────┬─────┘  └────┬─────┘  └───────┬───────┘ │
│       │              │                │          │
│       └──────────────┼────────────────┘          │
│                      │ REST API                  │
└──────────────────────┼───────────────────────────┘
                       │
┌──────────────────────┼───────────────────────────┐
│              Backend (API Server)                  │
│  ┌───────────┐  ┌────────────┐  ┌─────────────┐ │
│  │ Bot CRUD  │  │ Tournament │  │ Game Engine  │ │
│  │ API       │  │ Manager    │  │ (Lua 5.1)   │ │
│  └─────┬─────┘  └─────┬──────┘  └──────┬──────┘ │
│        │               │                │         │
│        └───────────────┼────────────────┘         │
│                        │                          │
│                   ┌────┴────┐                     │
│                   │ SQLite  │                     │
│                   │   DB    │                     │
│                   └─────────┘                     │
└───────────────────────────────────────────────────┘
```

## Technology Stack

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| Frontend | React + TypeScript | Modern, well-supported SPA framework |
| Code Editor | Monaco Editor | VS Code editor in browser, excellent Lua support |
| Game Renderer | PixiJS | Sprite-based 2D rendering using original game sprites |
| Backend | Rust (Axum) | Memory-safe Lua sandboxing via mlua, real Lua 5.1 compatibility |
| Lua Runtime | Lua 5.1 via mlua | Exact compatibility with original bot API, built-in sandbox mode |
| Database | SQLite | Simple, no separate server, perfect for MVP |
| Build/Dev | `just` (justfile) | Simple task runner for dev workflow |
| API | REST (JSON) | Simple, well-understood for CRUD operations |
| Real-time | WebSocket | Stream game state to browser for live rendering |

### Why server-side Lua (not browser-side)?

Running Lua in the browser (via WASM) is possible but introduces problems:
- Hard to enforce CPU/memory limits per player
- Game simulation integrity - all bots must run in the same tick-synchronous loop
- The original game ran Lua server-side for good reason (isolation, fairness)
- Server-side Lua 5.1 gives **exact** compatibility with original bots

The browser handles rendering and editing only. The server runs the actual game.

## Components

### 1. Game Engine (Server-Side)

Port the core game logic from C to a server-side implementation. Key subsystems:

#### 1a. World System
- Tile grid with food, walkability, graphics type
- Load maps from original level format (Lua files)
- Food regeneration per tick
- A* pathfinding

#### 1b. Creature System
- All 3 creature types with original stats
- State machine: IDLE, WALK, EAT, HEAL, ATTACK, CONVERT, SPAWN, FEED
- Aging (health drain)
- Movement along paths
- Combat with original damage table

#### 1c. Player/Bot Runtime
- Per-player Lua 5.1 VM (isolated)
- Load oo.lua and state.lua APIs (directly from original source)
- CPU/memory limits per player
- Coroutine-based creature execution
- All original low-level API functions exposed

#### 1d. Game Loop
- 100ms tick rate (configurable)
- Each tick: rules -> world update -> player think -> creature processing
- Game state serialization for client updates

#### 1e. Rules Engine
- Load original rules from Lua
- Scoring, win conditions, map rotation

### 2. Bot Management API (REST)

Simple CRUD for bot management:

```
GET    /api/bots              # List all bots
POST   /api/bots              # Create new bot
GET    /api/bots/:id          # Get bot details
PUT    /api/bots/:id          # Update bot
DELETE /api/bots/:id          # Delete bot

GET    /api/bots/:id/versions          # List versions
POST   /api/bots/:id/versions          # Save new version
GET    /api/bots/:id/versions/:vid     # Get specific version

GET    /api/tournaments                 # List tournaments
POST   /api/tournaments                 # Create tournament
GET    /api/tournaments/:id             # Get tournament details
POST   /api/tournaments/:id/entries     # Add bot entry
DELETE /api/tournaments/:id/entries/:eid # Remove entry
POST   /api/tournaments/:id/run         # Start tournament match
GET    /api/tournaments/:id/results     # Get results
```

### 3. Game State Streaming (WebSocket)

Stream game visualization data to the browser:

```
WS /ws/game/:id

Server -> Client messages:
  { type: "round", time: 12300, delta: 100 }
  { type: "creatures", data: [{ id, x, y, type, state, health, food, player, message }] }
  { type: "world", tiles: [{ x, y, food }] }  // only changed tiles
  { type: "players", data: [{ id, name, score, color }] }
  { type: "koth", player_id: 3 }
  { type: "game_end", winner: 2 }
```

### 4. Frontend

#### 4a. Bot Editor Page
- Monaco editor with Lua syntax highlighting
- Bot metadata (name, description)
- Save / Save as new version
- Test bot (run solo in a quick game)
- Lua API reference sidebar/panel

#### 4b. Bot Library Page
- List all bots with name, last modified, version count
- Create new bot
- Delete bot
- View version history per bot
- Duplicate/fork a bot

#### 4c. Tournament Page
- Create new tournament
- Add bot entries (pick bot + version)
- Can add same bot multiple times (different versions)
- Start match
- Watch match live (game canvas)
- View results (scores, replay)

#### 4d. Game Renderer
- PixiJS with original game sprites
- Render tile map (walls, ground, food levels)
- Render creatures (colored by player, type determines sprite)
- Show creature messages
- Show health/food bars
- King of the Hill indicator
- Score overlay
- Minimap (optional)

### 5. Database Schema (SQLite)

```sql
CREATE TABLE bots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT DEFAULT '',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE bot_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bot_id INTEGER NOT NULL REFERENCES bots(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    code TEXT NOT NULL,
    api_type TEXT NOT NULL DEFAULT 'oo',  -- 'oo' or 'state'
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(bot_id, version)
);

CREATE TABLE tournaments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    status TEXT DEFAULT 'created',  -- created, running, finished
    map TEXT DEFAULT 'default',
    config TEXT DEFAULT '{}',  -- JSON game config overrides
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE tournament_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tournament_id INTEGER NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    bot_version_id INTEGER NOT NULL REFERENCES bot_versions(id),
    slot_name TEXT DEFAULT '',  -- display name in match
    UNIQUE(tournament_id, id)
);

CREATE TABLE tournament_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tournament_id INTEGER NOT NULL REFERENCES tournaments(id) ON DELETE CASCADE,
    player_slot INTEGER NOT NULL,
    bot_version_id INTEGER NOT NULL REFERENCES bot_versions(id),
    final_score INTEGER DEFAULT 0,
    creatures_spawned INTEGER DEFAULT 0,
    creatures_killed INTEGER DEFAULT 0,
    creatures_lost INTEGER DEFAULT 0,
    finished_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

## Implementation Phases

### Phase 1: Game Engine Core
**Goal**: Port the game simulation to run server-side with Lua 5.1 compatibility.

1. Set up project structure and `justfile`
2. Implement world/tile system (load original maps)
3. Implement creature system (types, states, stats from original)
4. Embed Lua 5.1, expose low-level API functions
5. Load original `oo.lua` and `state.lua` APIs unmodified
6. Implement game loop (100ms ticks)
7. **Validation**: Run original example bots (stupibot, easybot) and verify they work

### Phase 2: API Server & Database
**Goal**: Bot storage and tournament management.

1. Set up REST API server
2. Set up SQLite database with schema
3. Implement bot CRUD endpoints
4. Implement bot versioning
5. Implement tournament CRUD endpoints
6. **Validation**: Can create, save, version, and list bots via API

### Phase 3: Frontend - Editor & Library
**Goal**: Browser-based bot editing and management.

1. Set up React + TypeScript project
2. Integrate Monaco editor with Lua highlighting
3. Build bot library page (list, create, delete)
4. Build bot editor page (edit, save, version)
5. Wire up to REST API
6. **Validation**: Can write, save, and manage bots in browser

### Phase 4: Game Visualization
**Goal**: Watch games live in the browser.

1. Implement WebSocket game state streaming
2. Build game renderer (Canvas/PixiJS)
3. Render map, creatures, food, scores
4. "Test bot" feature (run single bot solo)
5. **Validation**: Can write a bot and watch it run

### Phase 5: Tournament System
**Goal**: Set up and run multi-bot matches.

1. Build tournament page UI
2. Add bot entries to tournaments
3. "Run match" triggers server-side game with selected bots
4. Stream match to browser
5. Display results and scores
6. **Validation**: Can create tournament, add bots, run match, see results

### Phase 6: Polish & Quality of Life
- Lua API reference panel in editor
- Creature info tooltips in renderer
- Game speed controls (pause, fast-forward)
- Basic error display (Lua syntax/runtime errors shown in editor)
- Map selection for tournaments
- Match replay (store game state log, replay in renderer)

## Justfile

```just
# Development task runner

# Start everything for development
dev: dev-backend dev-frontend

# Start backend server
dev-backend:
    cd backend && cargo run --bin server

# Start frontend dev server
dev-frontend:
    cd frontend && npm run dev

# Run database migrations
db-migrate:
    cd backend && cargo run --bin migrate

# Reset database
db-reset:
    rm -f backend/swarmcrest.db
    just db-migrate

# Run backend tests
test-backend:
    cd backend && cargo test

# Run frontend tests
test-frontend:
    cd frontend && npm test

# Run all tests
test: test-backend test-frontend

# Build for production
build:
    cd backend && cargo build --release
    cd frontend && npm run build

# Validate bot compatibility (run original bots through engine)
validate-bots:
    cd backend && cargo test --package engine -- original_bots --nocapture
```

## Compatibility Strategy

To ensure original bots "just work":

1. **Use Lua 5.1** - Same version as original (not 5.4 or LuaJIT)
2. **Copy `oo.lua` and `state.lua` verbatim** - Load the original high-level API files unmodified
3. **Match low-level API exactly** - Same function names, same parameter order, same return values
4. **Match game constants** - Same creature stats, speeds, costs, damage values
5. **Match tick timing** - 100ms ticks, same aging/healing/eating rates
6. **Validation test suite** - Run every original example bot and verify behavior matches

Functions to implement with exact signatures:
```lua
-- These must match the original exactly
set_path(id, x, y)
set_state(id, state)
get_state(id)
set_target(id, target_id)
set_convert(id, type)
suicide(id)
get_pos(id)
get_type(id)
get_food(id)
get_health(id)
get_speed(id)
get_tile_food(id)
get_tile_type(id)
get_max_food(id)
get_distance(id1, id2)
get_nearest_enemy(id)
set_message(id, msg)
creature_exists(id)
creature_player(id)
world_size()
game_time()
get_koth_pos()
player_exists(id)
king_player()
player_score(id)
get_cpu_usage()
print(msg)
```

## Decisions

1. **Backend language**: Rust + mlua. Real Lua 5.1 C engine with safe Rust FFI wrappers. Best combination of exact bot compatibility and memory-safe sandboxing for untrusted player code.
2. **Renderer**: PixiJS. Using original game sprites, PixiJS provides efficient sprite batching and a mature 2D rendering pipeline.
3. **Map format**: JSON. Convert original Lua maps to a JSON format. Server loads JSON maps into the game engine - simpler than running Lua map scripts, and the frontend can fetch the same JSON for rendering.
4. **Game speed**: Games default to headless mode (fast compute, no per-tick delay). Pass `headless: false` to run in real-time mode (100ms/tick) for live viewing via WebSocket. The web UI sends `headless: false` when the "Headless" checkbox is unchecked.
5. **Multiple simultaneous games**: MVP supports one game per server. Architecture should keep game state isolated (no globals) so multiple games can be added later.

## File Structure

```
infon/
  docs/                    # Documentation (this file, API reference, etc.)
  orig_game/               # Original C/Lua source (reference)
  backend/
    cmd/
      server/main.go       # Server entry point
      migrate/main.go       # DB migration tool
    engine/
      world.go             # Tile grid, pathfinding, food
      creature.go          # Creature types, states, combat
      player.go            # Player management, Lua VM
      game.go              # Game loop orchestration
      lua_api.go           # Low-level Lua API bindings
      rules.go             # Game rules
    api/
      server.go            # HTTP server setup
      bots.go              # Bot CRUD handlers
      tournaments.go       # Tournament handlers
      websocket.go         # Game state streaming
    db/
      schema.sql           # Database schema
      db.go                # Database access layer
    lua/
      oo.lua               # Copied from orig_game/api/oo.lua
      state.lua            # Copied from orig_game/api/state.lua
    swarmcrest.db           # SQLite database (gitignored)
  frontend/
    src/
      components/
        Editor.tsx          # Monaco Lua editor
        BotLibrary.tsx      # Bot list and management
        GameCanvas.tsx      # Game renderer
        Tournament.tsx      # Tournament page
      api/
        client.ts           # REST API client
      App.tsx
    package.json
  justfile                 # Development task runner
  README.md
```
