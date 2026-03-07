# SwarmCrest - Game Overview

SwarmCrest is an open-source (GPL) multiplayer programming game where players write Lua scripts to control creatures that compete for food and survival. Originally created by Florian Wesch, it's described as "like CoreWars on steroids."

## How It Works

Players connect to a server, upload Lua code, and their creatures autonomously execute that code in a shared game world. The game operates in **100ms rounds** (ticks). Each round, every creature's Lua coroutine is resumed, allowing it to make decisions and take actions.

## Game World

- 2D tile-based grid. Each tile is 256x256 units.
- X increases rightward, Y increases downward.
- Minimum coordinates start at 256,256 (border tiles are non-walkable).
- `world_size()` returns the playable boundaries as `x1, y1, x2, y2`.
- Tiles are either `TILE_SOLID` (walls) or `TILE_PLAIN` (walkable).
- Food spawns on tiles and can be consumed by creatures.

## Creature Types

There are 3 creature types with distinct roles:

### Type 0 - Small (Balanced)
| Stat | Value |
|------|-------|
| Speed | 200-300 units/sec (200 + health/16) |
| Max Food | 10,000 |
| Hitpoints | 10,000 |
| Attack Range | 768 units (vs Type 2 only) |
| Health Drain | 50 HP/sec |
| Healing Rate | 500 HP/sec |
| Eating Rate | 800 food/sec |
| Feeding Rate | 400 food/sec |
| Converts To | Type 1 (8,000 food) or Type 2 (5,000 food) |

### Type 1 - Big (Tank)
| Stat | Value |
|------|-------|
| Speed | 400 units/sec |
| Max Food | 20,000 |
| Hitpoints | 20,000 |
| Attack Range | 512 units (all types) |
| Health Drain | 70 HP/sec |
| Healing Rate | 300 HP/sec |
| Eating Rate | 400 food/sec |
| Converts To | Type 0 (8,000 food) |
| Spawning | Spawns Type 0; costs 5,000 food + 20% health |

### Type 2 - Flyer (Scout)
| Stat | Value |
|------|-------|
| Speed | 800 units/sec (fastest) |
| Max Food | 5,000 |
| Hitpoints | 5,000 |
| Attack | Cannot attack |
| Health Drain | 50 HP/sec |
| Healing Rate | 600 HP/sec |
| Eating Rate | 600 food/sec |
| Feeding Rate | 400 food/sec |
| Converts To | Type 0 (5,000 food) |
| Special | Can fly over water and walls |

## Creature States

| State | Description |
|-------|-------------|
| `CREATURE_IDLE` | Doing nothing. Required to be King of the Hill. |
| `CREATURE_WALK` | Moving to destination set by `set_path`. |
| `CREATURE_HEAL` | Using food to restore health. |
| `CREATURE_EAT` | Consuming food from the current tile. |
| `CREATURE_ATTACK` | Attacking a target creature. |
| `CREATURE_CONVERT` | Changing creature type (costs food). |
| `CREATURE_SPAWN` | Type 1 producing a new Type 0 creature. |
| `CREATURE_FEED` | Transferring food to another creature. Max distance: 256 units. |

## Combat

Damage depends on attacker and defender types:

| Attacker | vs Small | vs Big | vs Flyer |
|----------|----------|--------|----------|
| Small | 0 | 0 | 1000 |
| Big | 1500 | 1500 | 1500 |
| Flyer | - | - | - |

- Flyers cannot attack.
- Small creatures can only attack Flyers.
- Big creatures can attack everything.

## King of the Hill

- A special tile on the map at a fixed position.
- A creature in `CREATURE_IDLE` state on that tile becomes king.
- The king's player scores points each tick.
- `king_player()` returns the current king's player ID.

## Scoring

Points are earned and lost through several mechanisms:

### King of the Hill
- A creature in `CREATURE_IDLE` state on the King of the Hill tile scores for its player.
- +30 points awarded for every 10 seconds of continuous holding.
- If the king player changes, the timer resets.
- If multiple players have creatures on the tile simultaneously, no one scores and the timer resets.
- `king_player()` returns the current king's player ID.

### Kill & Death Points
| Event | Points |
|-------|--------|
| Spawning a creature | +10 to parent's player |
| Killing a Small (Type 0) | +10 to killer, -3 to victim |
| Killing a Big (Type 1) | +15 to killer, -8 to victim |
| Killing a Flyer (Type 2) | +12 to killer, -4 to victim |
| Creature starvation | -3 to owner |
| Creature suicide | -40 to owner |

## Win Conditions

A match can end in three ways (checked in priority order):

1. **Score Limit** (default: 500 points) — The first player to reach the score limit wins immediately.
2. **Last Player Standing** — If all of one player's creatures are eliminated, the surviving player wins.
3. **Time Limit** (default: 10 minutes / 6,000 ticks) — When time expires, the player with the highest score wins. If the top players are tied, the match is a draw.

## Creature Lifecycle

1. **Spawning**: A Type 1 creature uses food and health to create a new Type 0 creature.
2. **Aging**: All creatures continuously lose health (type-dependent drain rate).
3. **Eating**: Creatures eat food from tiles to sustain themselves.
4. **Converting**: Type 0 can convert to Type 1 or 2 using food.
5. **Death**: Health reaches 0 (starvation or combat), or suicide.
6. **Food Drop**: When a creature dies by suicide, 1/3 of its food drops on the tile.

## Original Architecture

- **Server** (`infond`): C application with embedded Lua 5.1.2
- **Client**: SDL/OpenGL 2D/3D renderer, ASCII art, or telnet
- **Networking**: TCP on port 1234, libevent-based, with optional zlib compression
- **Per-player Lua VM**: Each player has an isolated Lua state with CPU/memory limits
- **Pathfinding**: A* for ground creatures; direct flight for Flyers
