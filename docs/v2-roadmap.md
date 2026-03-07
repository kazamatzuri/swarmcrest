# SwarmCrest — v2.0 Roadmap

## Overview

Version 2.0 transforms SwarmCrest from a single-user MVP into a full multiplayer competitive platform. Players sign up, write bots, compete in ranked matches and tournaments, and climb leaderboards — all through a web UI or a well-documented API that's LLM-friendly by design.

This document is organized into **phases**. Each phase is self-contained and delivers user-facing value. All phases will be developed in parallel.

---

## Phase 1: Accounts & Authentication

**Goal**: Multi-user support with proper identity, ownership, and access control.

### 1.1 Authentication

- **Email/password auth** — registration, login, password reset via email confirmation
- **OAuth providers** — Google, GitHub, Facebook (add incrementally; GitHub first since the audience is programmers)
- Session management via HTTP-only secure cookies (or JWTs with refresh tokens)
- Rate limiting on auth endpoints

### 1.2 User Accounts

- Profile: display name, avatar (Gravatar or upload), bio, join date
- Account settings: change password, link/unlink OAuth providers, manage API tokens
- Account deletion (with data export)

### 1.3 Authorization & Ownership

- All bots belong to a user
- All API endpoints become user-scoped
- Admin role for platform management
- Bot visibility: **public** (source visible to all) vs **private** (source visible only to creator)

### Database Changes

```
users
  id            UUID PRIMARY KEY
  username      TEXT UNIQUE NOT NULL
  email         TEXT UNIQUE NOT NULL
  password_hash TEXT          -- NULL for OAuth-only users
  display_name  TEXT
  avatar_url    TEXT
  bio           TEXT
  role          TEXT DEFAULT 'user'   -- 'user' | 'admin'
  created_at    TIMESTAMP
  updated_at    TIMESTAMP

oauth_connections
  id            UUID PRIMARY KEY
  user_id       UUID REFERENCES users(id)
  provider      TEXT NOT NULL          -- 'google' | 'github' | 'facebook'
  provider_id   TEXT NOT NULL
  UNIQUE(provider, provider_id)

-- Add to existing bots table:
  owner_id      UUID REFERENCES users(id)
  visibility    TEXT DEFAULT 'public'  -- 'public' | 'private'
```

### Migration Note

The current SQLite database has no user concept. Migration should:
1. Create the users table
2. Create a default "legacy" user for any existing bots
3. Add owner_id to bots table with foreign key to users

---

## Phase 2: Bot Versioning & Elo Rating System

**Goal**: Automatic versioning on every edit, per-version Elo ratings, and the ranking foundation for leaderboards.

### 2.1 Strict Bot Versioning

Every save of a bot's code creates a new immutable version. Versions cannot be edited — only new versions can be created. Users can:

- **Archive** old versions (hidden from selection UIs but still referenced by match history)
- **Rename** bots (name is on the bot, not the version)
- **Set active version** — the default version used when entering matches
- View full version history with diffs between versions

### 2.2 Elo Rating System

Elo ratings apply to **1v1 matches** and **2v2 team matches** only. FFA games use placement-based scoring without Elo (see Phase 3).

Each **bot version** has its own Elo rating, independent from other versions of the same bot. New versions use a **soft reset**: `(parent_version_elo + 1500) / 2`, which starts closer to the parent's proven skill without fully inheriting a rating earned by different code.

#### Core Parameters

| Parameter | Value | Notes |
|-----------|-------|-------|
| Starting Elo | 1500 | Industry standard (FIDE, USCF, LMSYS, etc.) |
| K-factor (provisional) | 40 | First 30 games of a version — fast convergence |
| K-factor (established) | 20 | After 30 games, rating < 2400 |
| K-factor (elite) | 10 | Rating ≥ 2400 — stabilize top of leaderboard |
| Rating floor | 100 | Prevent negative ratings |

#### 1v1 Matches

Standard Elo formula:
- Expected score: `E_a = 1 / (1 + 10^((R_b - R_a) / 400))`
- New rating: `R'_a = R_a + K * (S_a - E_a)` where S is 1 (win), 0.5 (draw), 0 (loss)

#### Team Matches (2v2)

Each **team** (a named pair of bot versions) has its own Elo rating. The team average rating is used in the standard 1v1 formula against the opposing team's average. To prevent rating farming (e.g., a 2000-rated bot teaming with a 1000-rated bot to face weaker opponents), each bot's individual rating update is computed based on *its own rating* versus the *enemy team average* — so the strong bot gains very little from an expected win, while the weaker teammate gains more. Consider widening the scaling divisor from 400 to 500 for team games to dampen individual rating variance.

#### Free-For-All — No Elo

FFA games do **not** use Elo ratings. Pairwise Elo decomposition for multiplayer is fundamentally flawed (players who happen to avoid each other get inflated ratings). Instead, FFA uses placement-based scoring:
- Players are ranked by final score at end of match
- Placement points awarded: 1st = N points, 2nd = N-1, ..., last = 1 (where N = number of players)
- FFA leaderboard ranked by cumulative placement points and average placement
- This is simpler, fairer, and avoids the well-known problems with multiplayer Elo decomposition

### 2.3 Stats Tracking

Per bot version:
- Elo rating + peak Elo (1v1 and 2v2 only), game count
- Win/loss/draw record (by match format: 1v1, FFA, 2v2)
- FFA: average placement, total placement points
- Creatures spawned/killed/lost totals
- Average score per game

Per user:
- Number of bots, total games played
- Best Elo across all bot versions (1v1 + 2v2)
- Best FFA average placement

---

## Phase 3: Leaderboards

**Goal**: Public rankings that drive competition, with a separate space for newcomers.

### 3.1 Leaderboard Types

Three separate leaderboards:
1. **1v1** — ranked by bot version Elo
2. **FFA** — ranked by cumulative placement points / average placement (no Elo)
3. **2v2 Teams** — ranked by team Elo

Each leaderboard shows: rank, bot name, version, owner, rating (Elo or placement score), games played, win rate.

### 3.2 Leagues by Account Age

To give newcomers a fair playground:

| League | Account Age | Description |
|--------|------------|-------------|
| **Newcomer** | 0–14 days | Protected space; newcomer bots only face each other in ranked |
| **Open** | 15+ days | The main competitive league |

- Newcomers can optionally challenge Open league bots (unranked for the Open bot)
- After 14 days, Newcomer bots and their Elo carry over into Open league
- Leaderboards are displayed per-league with an "All" view

### 3.3 Leaderboard Features

- Filterable by league, match format, time period (all-time, this month, this week)
- Clickable entries → bot profile page with stats and match history
- Historical Elo graph per bot version
- "Rising" indicator for bots on a win streak

---

## Phase 4: Match System

**Goal**: A system for running matches on shared infrastructure, protected by rate limiting.

### 4.1 Rate Limiting

Instead of a token economy, match abuse is prevented via per-user rate limits:

| Limit | Value | Notes |
|-------|-------|-------|
| Concurrent live games | 3 per user | Prevents hogging live-stream slots |
| Live challenges per hour | 10 per user | Generous for active play |
| Headless challenges per hour | 30 per user | Fast iteration for bot development |
| Tournament entries per day | 5 per user | Prevents spam entries |

Rate limits are enforced server-side and communicated via standard `429 Too Many Requests` + `Retry-After` headers. Monetization will be explored separately down the road.

### 4.2 Challenge System

Players can challenge any bot from the leaderboard:

#### Live Challenge
1. Player selects their bot version and the opponent bot
2. Match enters the **game queue**
3. Player gets a match page that shows:
   - Queue position and rough ETA while waiting
   - Live game stream once the match starts
   - Results and replay link after completion
4. 1v1 and 2v2 matches affect Elo; FFA matches award placement points

#### Headless Challenge
1. Same selection flow
2. Match runs in background at maximum tick speed (no rendering overhead)
3. Player gets a notification when complete
4. Results page with replay link
5. Ratings updated normally

### 4.3 Game Queue

- FIFO queue
- Queue status visible to all users (current queue depth, estimated wait)
- Concurrent match runner (configurable parallelism based on server capacity)
- Match timeout: games have a maximum tick count to prevent infinite games

### 4.4 Public Games

Recurring open matches anyone can join:
- Reset every **2 hours**
- Up to **20 players** per game (may scale to more once engine optimizations in Phase 9 are proven)
- Larger maps proportional to player count
- Results count toward FFA placement leaderboard
- Live-streamed by default — spectate from the game list

---

## Phase 5: Replay System

**Goal**: Every match is recorded, replayable, and shareable.

### 5.1 Replay Format

Each match produces a replay file:

```
Match metadata:
  match_id:       UUIDv4
  format:         "1v1" | "ffa5" | "team2v2" | "public"
  map:            map identifier
  players:        [{ user_id, bot_id, bot_version_id, slot, color }]
  start_time:     ISO 8601
  tick_count:     total ticks
  version:        replay format version (for forward compatibility)

Tick data (binary or compressed JSON):
  Per tick:
    tick_number
    creature_updates: [{ id, x, y, type, state, health, food, player, message }]
    tile_updates:     [{ x, y, food }]   // only changed tiles
    score_updates:    [{ player_slot, score }]
    events:           [{ type, data }]   // kills, spawns, koth changes
```

Storage: compressed binary format (e.g., MessagePack + zstd). Keep raw for recent matches, archive to object storage after 30 days.

### 5.2 Replay Viewer

- Same game renderer used for live games, fed from replay data instead of WebSocket
- Playback controls: play, pause, speed (0.5x, 1x, 2x, 4x, 8x), seek via timeline scrubber
- Jump to key events (kills, scoring moments)
- Shareable URL: `/matches/{match_id}`

### 5.3 Match Pages

Every match has a permanent page:
- Match result summary (winner, scores, Elo changes)
- Participant list with bot versions
- Replay viewer
- Link to download replay file
- For tournaments: link back to tournament bracket

---

## Phase 6: Tournaments & Leagues

**Goal**: Structured competitive events from automated leagues to prize-money tournaments.

### 6.1 Regular Leagues

Automated recurring tournaments:
- **Daily ladder**: 1v1 Swiss-system, 5 rounds, top Elo bots auto-entered
- **Weekly FFA**: 5-player FFA bracket, open registration
- **Monthly championship**: Invite top N from weekly results

### 6.2 Tournament Structure

Tournaments support multiple formats:
- **Single elimination** — bracket, best-of-1 or best-of-3
- **Round robin** — every bot plays every other bot
- **Swiss system** — fixed rounds, opponents matched by current standing

Tournament lifecycle:
1. **Registration** — open for entries (subject to daily rate limit)
2. **Seeding** — based on current Elo (1v1/2v2) or placement score (FFA)
3. **Running** — matches execute in order, results update live
4. **Complete** — final standings, rating adjustments applied

### 6.3 Tournament Visualization

- **Bracket view** for elimination tournaments (interactive SVG/Canvas)
- **Standings table** for round robin / Swiss (live-updating)
- **Match links** from every bracket slot to the match replay
- **Tournament history** page listing past tournaments with winners

### 6.4 Prize-Money Tournaments — BACKLOG

Moved to backlog. Requires significant prerequisites before consideration:
- Stable user base (200+ monthly active players)
- Proven anti-cheat and rating integrity over 6+ months
- Legal entity, terms of service, tax reporting (1099 in US for prizes > $600)
- Age verification, regional legality checks (gambling law varies by jurisdiction)
- Payment integration (Stripe)
- Deterministic replay verification (money on the line = every result must be independently verifiable)

Will revisit once the competitive scene is established and there is organic community demand.

---

## Phase 7: Teams

**Goal**: Support 2v2 team play with named, versioned teams.

### 7.1 Team Structure

- A **team** is a named pairing of two bot versions, owned by a single user
- Teams are versioned: changing either bot version creates a new team version
- Each team version has its own Elo rating (independent of the individual bot Elos)
- Users can create multiple teams with different bot combinations

### Database

```
teams
  id              UUID PRIMARY KEY
  owner_id        UUID REFERENCES users(id)
  name            TEXT NOT NULL
  created_at      TIMESTAMP

team_versions
  id              UUID PRIMARY KEY
  team_id         UUID REFERENCES teams(id)
  version         INTEGER NOT NULL
  bot_version_a   UUID REFERENCES bot_versions(id)
  bot_version_b   UUID REFERENCES bot_versions(id)
  elo_rating      INTEGER DEFAULT 1200
  games_played    INTEGER DEFAULT 0
  created_at      TIMESTAMP
  UNIQUE(team_id, version)
```

### 7.2 Team Features

- Team management page: create, name, select bot versions, view history
- Team leaderboard (separate from individual)
- Team challenges (2v2 format)
- Team tournaments

---

## Phase 8: API & LLM Integration

**Goal**: A clean, well-documented API that humans and LLMs can use equally well.

### 8.1 API Token Management

- Users generate API tokens from their account settings
- Tokens have configurable scopes: `bots:read`, `bots:write`, `matches:read`, `matches:write`, `leaderboard:read`
- Token listing, revocation, and last-used tracking
- Rate limiting per token (e.g., 60 requests/minute)

### 8.2 API Endpoints (v2)

```
Authentication:
  POST   /api/v2/auth/register
  POST   /api/v2/auth/login
  POST   /api/v2/auth/oauth/{provider}
  POST   /api/v2/auth/refresh
  DELETE /api/v2/auth/logout

Users:
  GET    /api/v2/users/me
  PUT    /api/v2/users/me
  GET    /api/v2/users/{id}/profile     (public profile)

Bots:
  GET    /api/v2/bots                   (own bots)
  POST   /api/v2/bots
  GET    /api/v2/bots/{id}
  PUT    /api/v2/bots/{id}              (rename, change visibility)
  DELETE /api/v2/bots/{id}
  GET    /api/v2/bots/{id}/versions
  POST   /api/v2/bots/{id}/versions     (create new version / save code)
  GET    /api/v2/bots/{id}/versions/{v}
  PUT    /api/v2/bots/{id}/versions/{v} (archive/unarchive)
  GET    /api/v2/bots/{id}/stats

Teams:
  GET    /api/v2/teams
  POST   /api/v2/teams
  GET    /api/v2/teams/{id}
  PUT    /api/v2/teams/{id}
  GET    /api/v2/teams/{id}/versions

Matches:
  POST   /api/v2/matches/challenge      (create a challenge)
  GET    /api/v2/matches/{id}           (match result + replay link)
  GET    /api/v2/matches/{id}/replay    (download replay)
  WS     /api/v2/matches/{id}/stream    (live game stream)

Queue:
  GET    /api/v2/queue/status           (queue depth, ETA)

Leaderboards:
  GET    /api/v2/leaderboards/1v1
  GET    /api/v2/leaderboards/ffa
  GET    /api/v2/leaderboards/2v2
  Query params: ?league=newcomer|open&period=all|month|week&page=1&limit=50

Tournaments:
  GET    /api/v2/tournaments
  GET    /api/v2/tournaments/{id}
  POST   /api/v2/tournaments/{id}/enter
  GET    /api/v2/tournaments/{id}/bracket
  GET    /api/v2/tournaments/{id}/matches

API Keys:
  GET    /api/v2/api-keys
  POST   /api/v2/api-keys
  DELETE /api/v2/api-keys/{id}
```

### 8.3 LLM-Friendly Documentation

The API should be straightforward for LLMs to use on behalf of users:

- **OpenAPI 3.1 spec** — machine-readable, auto-generated from code
- **`llms.txt`** at site root — concise plain-text API overview following the llms.txt convention
- **`/api/v2/docs`** — interactive Swagger UI for humans
- **Example workflows** documented as step-by-step sequences:
  1. "Create a bot and submit it to a tournament"
  2. "Challenge the #1 leaderboard bot"
  3. "Check my bot's stats and recent matches"
- **Lua API reference** available via API endpoint (`GET /api/v2/docs/lua-api`) so LLMs can retrieve it programmatically
- **Error responses** use consistent JSON format with human-readable messages

### 8.4 LLM Workflow

A typical LLM-driven workflow:
1. User gives LLM their API token
2. LLM fetches `llms.txt` to understand the platform
3. LLM fetches Lua API docs to understand bot programming
4. LLM creates a bot, writes code, saves a version
5. LLM checks leaderboard, picks an opponent, submits a challenge
6. LLM polls for match result, analyzes replay data, iterates on bot code

---
--------------------------------------
## Phase 9: Scaling & Large Games

**Goal**: Optimize the engine for 20-player public games, with a path toward larger games if demand warrants it.

The initial target is **20 players** per public game. This is already visually impressive and avoids the unsolved game balance problems of 100-player games (food scarcity, KotH meaninglessness at scale, Lua VM budget). We can scale up incrementally once 20-player is proven.

### 9.1 Larger Maps

- New map format supporting larger tile grids (current maps are small)
- Map generator for large procedural maps with balanced resource distribution
- Maps sized proportionally to player count

### 9.2 Engine Optimization

- Profile and optimize the Lua VM pool (20+ concurrent VMs)
- Batch creature updates for efficiency
- Consider spatial partitioning (quadtree) for collision/nearest-enemy queries at scale
- Tick budget monitoring — if a tick takes too long, reduce per-player CPU allowance
- Headless mode (no WebSocket serialization) for non-streamed games runs much faster

### 9.3 Streaming Optimization

- Delta compression for WebSocket messages (only send changes)
- Viewport-based streaming (only send creatures visible to the viewer's viewport)
- Reduce update frequency for distant creatures


---

## Phase 10: Live Experience

**Goal**: Make watching matches a social, engaging experience.

### 10.1 Live Match Chat

- Chat room per active match
- Only participants (bot owners in that match) can post during live matches
- Spectators can chat during public games
- Chat history saved with match replay
- Basic moderation: mute, report, word filter

### 10.2 Notifications

- In-app notifications for:
  - Match completed (headless challenge results)
  - Tournament round starting
  - Bot challenged by another player
  - Weekly digest of bot performance
- Optional email notifications (configurable per type)
- WebSocket-based real-time notification delivery

### 10.3 Spectating

- Game list page showing all currently running matches
- Spectator count displayed on each match
- Public games prominently featured
- Tournament matches highlighted during tournament events

---

## Phase 11: Documentation & Content

**Goal**: Comprehensive docs for humans and machines, plus proper attribution.

### 11.1 Human Documentation

- **Getting Started** guide: sign up → write first bot → run first match
- **Lua API Reference**: existing docs, expanded with more examples
- **Strategy Guide**: creature types, common patterns, advanced tactics
- **FAQ / Troubleshooting**: common Lua errors, why my bot isn't moving, etc.
- **Tournament Guide**: how leagues work, Elo explained, rate limits

### 11.2 LLM Documentation

- **`llms.txt`**: platform overview, API summary, key concepts (at site root)
- **`llms-full.txt`**: complete API reference + Lua API + game mechanics in one file
- **OpenAPI spec**: machine-parseable REST API definition
- Both served as static files and available via API endpoints

### 11.3 About Page

- Credit to **Florian Wesch** as creator of the original Infon Battle Arena
- Link to original game page
- Link to original wiki
- Link to original source files / repository
- Brief history of the game and this web adaptation
- License information (GPL, matching original)

### 11.4 Feedback

- In-app feedback form (accessible from every page via a persistent button)
- Fields: category (bug, feature request, general), description, optional screenshot upload
- Feedback stored in database and optionally forwarded to email/issue tracker
- Public roadmap page showing planned features and their status

---

## Phase 12: Community

**Goal**: Build a space for players to discuss strategy, share bots, and grow the community.

### 12.1 Platform Evaluation

Discord's 2026 mandatory facial-scan/government-ID verification (following a 2025 data breach of ~70K credentials including previously submitted IDs) has made many communities actively seek alternatives. Guilded — once the best gaming-focused alternative — was shut down by Roblox at end of 2025. The landscape as of early 2026:

| Platform | Pros | Cons | Verdict |
|----------|------|------|---------|
| **Stoat** (formerly Revolt) | Open-source, privacy-focused, near-identical Discord UX, self-hostable | Young, small bot ecosystem, servers straining under recent user influx | **Best drop-in Discord replacement** |
| **Zulip** | Topic-based threading (great for technical discussion), search-indexable, used by Rust community | No voice chat, more academic feel, less "community hangout" vibe | **Best for organized technical discussion** |
| **Matrix/Element** | Decentralized, federated, E2E encrypted, bridges to Discord/Slack/IRC | Steeper learning curve, less polished client, heavier server requirements | Good for sovereignty-minded technical users |
| **GitHub Discussions** | Integrated with code, search-indexable, free, familiar to programmers | No real-time chat, limited formatting, no voice | Great for async technical discussion |
| **Built-in forums** | Full control, integrated with the platform, search-indexable | Development cost, moderation burden, no voice/video | Best long-term but expensive to build |

### 12.2 Recommended Approach

**Start with Stoat + GitHub Discussions**, evaluate as the community grows:

1. **Now**: **Stoat** server for real-time chat and community building (lowest migration friction from Discord, open-source, privacy-respecting)
2. **Now**: **GitHub Discussions** for technical topics, strategy sharing, and bug reports (search-indexable, persistent, already where programmers live)
3. **Later**: If organized technical threads become important, consider adding **Zulip** for structured strategy/development discussion
4. **Later**: Evaluate building lightweight in-app forums if the community outgrows external platforms
5. **Fallback**: Matrix/Element as self-hosted option if Stoat's scaling issues persist

### 12.3 Community Features (In-App)

Even without full forums, the platform itself should have:
- Public bot profiles with optional description and strategy notes
- Match comments (on replay pages)
- Player profiles showing public bots, stats, and tournament history
- "Featured match of the day" on the home page

---

## Phase 13: Local Play & Self-Hosting

**Goal**: Let players run games locally for faster iteration and testing.

### 13.1 Docker Container

Provide a Docker image that runs the full game server locally:

```bash
docker pull ghcr.io/swarmcrest/swarmcrest-server:latest
docker run -p 3000:3000 swarmcrest/swarmcrest-server
# Open http://localhost:3000
```

- Includes backend + frontend + SQLite
- No authentication required in local mode
- No rate limits in local mode
- Documented in a **"Local Development"** guide

### 13.2 CLI Tool (stretch goal)

A simple CLI for local bot testing:

```bash
swarmcrest test mybot.lua                    # Run bot solo on default map
swarmcrest match bot1.lua bot2.lua           # 1v1 match, print results
swarmcrest match --live bot1.lua bot2.lua    # 1v1 with local web viewer
```

---

## Dependencies

All phases are developed in parallel. The dependency graph below shows hard prerequisites — phases can otherwise proceed independently.

```
Phase 1: Accounts & Auth          ─── foundation for everything
  │
  ├── Phase 2: Versioning & Elo   ─── foundation for competition
  │     │
  │     ├── Phase 3: Leaderboards
  │     │
  │     ├── Phase 4: Match System
  │     │     │
  │     │     └── Phase 5: Replays
  │     │
  │     └── Phase 7: Teams
  │
  ├── Phase 6: Tournaments        ─── depends on matches + replays
  │
  ├── Phase 8: API & LLM          ─── grows with features
  │
  └── Phase 11: Documentation     ─── ongoing

Phase 9: Scaling (20+ players)    ─── independent engineering work
Phase 10: Live Experience         ─── after match system is solid
Phase 12: Community               ─── grows organically
Phase 13: Local Play              ─── independent, can ship anytime
```

---

## Database Migration Strategy

The current SQLite database is fine for MVP but v2.0 should evaluate **PostgreSQL** for:
- Concurrent writes (multiple simultaneous matches writing results)
- Better JSON support (for replay metadata, match configs)
- Full-text search (for bot/user search)
- Row-level security (for multi-tenant data isolation)

Migration path: keep SQLite for local/Docker mode, use PostgreSQL for the hosted platform. The `sqlx` crate already supports both.

---

## Cross-Cutting: Observability & Metrics (Prometheus/PromQL)

**Goal**: Expose platform and game metrics via Prometheus so we can build Grafana dashboards, set up alerts, and surface player-facing stats down the road.

This is not a phase — it's infrastructure that grows alongside every phase. The backend should expose a `/metrics` endpoint from day one.

### Metric Categories

#### Platform / Operational Metrics

```
# Gauges
swarmcrest_active_games                          # currently running games
swarmcrest_game_queue_depth                      # matches waiting to start
swarmcrest_connected_websockets                  # live WebSocket connections
swarmcrest_registered_users_total                # total user accounts
swarmcrest_lua_vm_pool_active                    # Lua VMs currently executing
swarmcrest_lua_vm_pool_available                 # Lua VMs idle in pool

# Counters
swarmcrest_games_started_total{format}           # by format: 1v1, ffa, 2v2, public
swarmcrest_games_completed_total{format}
swarmcrest_games_errored_total{format}           # games that crashed/timed out
swarmcrest_api_requests_total{method,endpoint,status}
swarmcrest_websocket_messages_sent_total
swarmcrest_bot_submissions_total                 # new bot versions created
swarmcrest_bot_validation_failures_total         # bots that failed validation

# Histograms
swarmcrest_game_duration_seconds{format}         # how long matches take
swarmcrest_game_tick_duration_ms                 # per-tick processing time
swarmcrest_lua_execution_duration_ms             # per-bot Lua execution time
swarmcrest_api_request_duration_seconds{endpoint}
swarmcrest_websocket_frame_size_bytes
```

#### Game / Gameplay Metrics

```
# Counters
swarmcrest_creatures_spawned_total{creature_type}     # bug, plant, koth_marker
swarmcrest_creatures_killed_total{creature_type}
swarmcrest_food_consumed_total
swarmcrest_koth_captures_total                        # king-of-the-hill flips

# Histograms
swarmcrest_match_score{format}                        # score distribution
swarmcrest_creatures_per_game{format}                 # total creatures spawned per match
swarmcrest_elo_change_per_match{format}               # magnitude of Elo swings (1v1, 2v2 only)
swarmcrest_ffa_placement{position}                    # distribution of FFA placements
```

#### Player-Facing Stats (derived via PromQL)

These queries power dashboards and eventually in-app stats pages:

```promql
# Games run today
increase(swarmcrest_games_completed_total[24h])

# Total kills across the platform (all time)
swarmcrest_creatures_killed_total

# Average game duration by format
histogram_quantile(0.5, rate(swarmcrest_game_duration_seconds_bucket[7d]))

# Most active format this week
topk(1, increase(swarmcrest_games_completed_total[7d]))

# Kill rate per minute across all games
rate(swarmcrest_creatures_killed_total[5m]) * 60

# Platform health: games erroring vs completing
rate(swarmcrest_games_errored_total[1h]) / rate(swarmcrest_games_completed_total[1h])

# Lua execution hot spots (P99 bot think time)
histogram_quantile(0.99, rate(swarmcrest_lua_execution_duration_ms_bucket[5m]))
```

### Implementation Notes

- Use the `prometheus` crate (or `metrics` + `metrics-exporter-prometheus`) in the Rust backend
- Expose `/metrics` endpoint alongside the API (separate port or path-based routing)
- Docker Compose gets a `prometheus` + `grafana` service for local dev
- Production: standard Prometheus scrape config, Grafana dashboards as code (JSON provisioning)
- **Labels matter**: use `format` (1v1/ffa/2v2/public), `creature_type`, `endpoint` labels consistently
- Keep cardinality low — do NOT use `user_id` or `bot_id` as Prometheus labels (use application-level queries for per-user stats)

### Grafana Dashboard Ideas

1. **Platform Overview** — active games, queue depth, WebSocket connections, API request rate
2. **Game Stats** — games/day, kills/day, average duration, format popularity
3. **Engine Health** — tick duration percentiles, Lua execution time, error rates
4. **Growth** — new users/day, new bots/day, matches/day trend

---

## Open Questions

1. **Map editor?** — Should users be able to create custom maps for private matches?
2. **Bot marketplace?** — Should there be a way to "fork" public bots (with attribution)?
3. **Seasonal resets?** — Should Elo reset periodically, or is an ever-growing rating history better?
4. **Spectator betting?** — Fun engagement mechanic with virtual currency (not real money)?
5. **Bot CPU limits** — Per-player CPU budget needs tuning as player counts grow. Start with ~5ms per bot per tick for 20-player games.
6. **Replay storage** — How long to keep replays? Compress and archive after 30 days? Keep tournament replays forever?
7. **Anti-cheat** — Beyond Lua sandboxing, do we need to detect bots that exploit engine bugs?
8. **Monetization** — Token system was dropped in favor of rate limiting. Future monetization options to explore: cosmetics (creature skins, profile badges), supporter tier with higher rate limits, sponsored tournaments.
