use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};

use rand::Rng;
use serde::Deserialize;

use super::config::*;

/// Parameters for random map generation.
pub struct RandomMapParams {
    pub width: usize,
    pub height: usize,
    pub wall_density: f64,
    pub food_amount: i32,
    pub num_food_spots: usize,
}

impl Default for RandomMapParams {
    fn default() -> Self {
        Self {
            width: 30,
            height: 30,
            wall_density: 0.35,
            food_amount: 50000,
            num_food_spots: 10,
        }
    }
}

/// A single tile in the world grid.
#[derive(Clone, Debug)]
pub struct Tile {
    pub tile_type: u8,
    pub gfx: u8,
    pub food: i32,
}

impl Default for Tile {
    fn default() -> Self {
        Tile {
            tile_type: TILE_SOLID,
            gfx: TILE_GFX_SOLID,
            food: 0,
        }
    }
}

/// Food spawner definition loaded from map JSON.
#[derive(Clone, Debug, Deserialize)]
pub struct FoodSpawner {
    pub x: usize,
    pub y: usize,
    pub radius: usize,
    pub amount: i32,
    pub interval: u32,
}

/// The game world: a 2D tile grid with food, pathfinding, and coordinate conversions.
pub struct World {
    pub width: usize,
    pub height: usize,
    tiles: Vec<Tile>,
    pub koth_x: usize,
    pub koth_y: usize,
    pub food_spawners: Vec<FoodSpawner>,
}

// --- JSON deserialization helpers ---

#[derive(Deserialize)]
struct MapJson {
    #[allow(dead_code)]
    name: Option<String>,
    width: usize,
    height: usize,
    koth_x: Option<usize>,
    koth_y: Option<usize>,
    tiles: Vec<TileJson>,
    food_spawners: Option<Vec<FoodSpawner>>,
}

#[derive(Deserialize)]
struct TileJson {
    x: usize,
    y: usize,
    #[serde(rename = "type")]
    tile_type: u8,
    gfx: Option<u8>,
}

// --- A* internals ---

#[derive(Copy, Clone, Eq, PartialEq)]
struct AStarNode {
    cost: i32,
    x: usize,
    y: usize,
}

impl Ord for AStarNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.cmp(&self.cost) // min-heap via reversed ordering
    }
}

impl PartialOrd for AStarNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl World {
    /// Create a new world with all tiles solid and no food.
    pub fn new(width: usize, height: usize) -> Self {
        World {
            width,
            height,
            tiles: vec![Tile::default(); width * height],
            koth_x: width / 2,
            koth_y: height / 2,
            food_spawners: Vec::new(),
        }
    }

    /// Generate a balanced map sized for the given player count.
    ///
    /// Maps scale proportionally:
    /// - 2 players: ~30x30
    /// - 4 players: ~50x50
    /// - 8+ players: ~80x80
    /// - 16+ players: ~100x100
    ///
    /// Features balanced resource distribution, walls around edges, KOTH in center,
    /// and spawn points spread around the map.
    pub fn generate_map(player_count: usize) -> Self {
        let player_count = player_count.max(2);

        // Scale map size with player count
        let base_size = 30;
        let size = match player_count {
            0..=2 => base_size,
            3..=4 => 50,
            5..=8 => 80,
            9..=16 => 100,
            _ => (100 + (player_count - 16) * 2).min(150),
        };

        // More food for more players
        let food_amount = 50000 + (player_count as i32 - 2) * 15000;
        let num_food_spots = (10 + player_count * 2).min(40);

        let params = RandomMapParams {
            width: size,
            height: size,
            wall_density: 0.30, // slightly less walls for larger maps
            food_amount,
            num_food_spots,
        };

        let mut world = Self::generate_random(params);

        // Add spawn points spread around the map for each player
        // Place them in a ring around the center, evenly spaced
        let center_x = size / 2;
        let center_y = size / 2;
        let ring_radius = (size as f64 * 0.35) as usize; // 35% of map size

        let walkable: Vec<(usize, usize)> = (1..size - 1)
            .flat_map(|x| (1..size - 1).map(move |y| (x, y)))
            .filter(|&(x, y)| world.is_walkable(x, y))
            .collect();

        for i in 0..player_count {
            let angle = 2.0 * std::f64::consts::PI * (i as f64) / (player_count as f64);
            let target_x = center_x as f64 + ring_radius as f64 * angle.cos();
            let target_y = center_y as f64 + ring_radius as f64 * angle.sin();

            // Find nearest walkable tile to the target spawn position
            if let Some(&(sx, sy)) = walkable.iter().min_by_key(|&&(wx, wy)| {
                let dx = wx as f64 - target_x;
                let dy = wy as f64 - target_y;
                (dx * dx + dy * dy) as usize
            }) {
                // Add a food spawner near each spawn point so players start near food
                world.food_spawners.push(FoodSpawner {
                    x: sx,
                    y: sy,
                    radius: 3,
                    amount: 800,
                    interval: 3000,
                });
            }
        }

        world
    }

    /// Generate a random world using cellular automata for organic wall patterns.
    ///
    /// Algorithm:
    /// 1. Create world with solid border, plain interior
    /// 2. Randomly seed ~wall_density fraction of interior as solid
    /// 3. Run 5 iterations of cellular automata smoothing
    /// 4. Flood-fill to find largest connected walkable region; fill smaller regions with solid
    /// 5. Place KOTH at nearest walkable tile to map center
    /// 6. Scatter food spawners on random walkable tiles
    #[allow(clippy::needless_range_loop)]
    pub fn generate_random(params: RandomMapParams) -> Self {
        let width = params.width.clamp(20, 150);
        let height = params.height.clamp(20, 150);
        let wall_density = params.wall_density.clamp(0.0, 0.6);
        let mut rng = rand::thread_rng();

        // Step 1 & 2: Start with all plain interior, then seed walls randomly
        let mut grid = vec![vec![TILE_SOLID; width]; height];
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                if rng.gen::<f64>() < wall_density {
                    grid[y][x] = TILE_SOLID;
                } else {
                    grid[y][x] = TILE_PLAIN;
                }
            }
        }

        // Step 3: Cellular automata smoothing (5 iterations)
        for _ in 0..5 {
            let mut new_grid = grid.clone();
            for y in 1..height - 1 {
                for x in 1..width - 1 {
                    let mut solid_neighbors = 0;
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let nx = (x as i32 + dx) as usize;
                            let ny = (y as i32 + dy) as usize;
                            if grid[ny][nx] == TILE_SOLID {
                                solid_neighbors += 1;
                            }
                        }
                    }
                    if solid_neighbors >= 5 {
                        new_grid[y][x] = TILE_SOLID;
                    } else if solid_neighbors <= 3 {
                        new_grid[y][x] = TILE_PLAIN;
                    }
                    // 4 neighbors: keep current state
                }
            }
            grid = new_grid;
        }

        // Step 4: Flood-fill to find the largest connected walkable region
        let mut visited = vec![vec![false; width]; height];
        let mut regions: Vec<Vec<(usize, usize)>> = Vec::new();

        for y in 1..height - 1 {
            for x in 1..width - 1 {
                if grid[y][x] == TILE_PLAIN && !visited[y][x] {
                    // BFS flood fill
                    let mut region = Vec::new();
                    let mut queue = VecDeque::new();
                    queue.push_back((x, y));
                    visited[y][x] = true;

                    while let Some((cx, cy)) = queue.pop_front() {
                        region.push((cx, cy));
                        for &(dx, dy) in &[(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                            let nx = (cx as i32 + dx) as usize;
                            let ny = (cy as i32 + dy) as usize;
                            if nx >= 1
                                && nx < width - 1
                                && ny >= 1
                                && ny < height - 1
                                && grid[ny][nx] == TILE_PLAIN
                                && !visited[ny][nx]
                            {
                                visited[ny][nx] = true;
                                queue.push_back((nx, ny));
                            }
                        }
                    }
                    regions.push(region);
                }
            }
        }

        // Find the largest region
        let largest_idx = regions
            .iter()
            .enumerate()
            .max_by_key(|(_, r)| r.len())
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Convert all non-largest regions to solid
        let mut largest_set = std::collections::HashSet::new();
        if !regions.is_empty() {
            for &(x, y) in &regions[largest_idx] {
                largest_set.insert((x, y));
            }
        }
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                if grid[y][x] == TILE_PLAIN && !largest_set.contains(&(x, y)) {
                    grid[y][x] = TILE_SOLID;
                }
            }
        }

        // Build the actual World from the grid
        let mut world = World::new(width, height);
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                if grid[y][x] == TILE_PLAIN {
                    world.set_type(x, y, TILE_PLAIN);
                }
            }
        }

        // Step 5: Place KOTH at nearest walkable tile to map center
        let center_x = width / 2;
        let center_y = height / 2;
        let mut best_koth = (center_x, center_y);
        let mut best_dist = usize::MAX;
        if !regions.is_empty() {
            for &(x, y) in &regions[largest_idx] {
                let dx = x.abs_diff(center_x);
                let dy = y.abs_diff(center_y);
                let dist = dx * dx + dy * dy;
                if dist < best_dist {
                    best_dist = dist;
                    best_koth = (x, y);
                }
            }
        }
        world.koth_x = best_koth.0;
        world.koth_y = best_koth.1;

        // Step 6: Scatter food spawners on random walkable tiles
        if !regions.is_empty() && !regions[largest_idx].is_empty() {
            let walkable = &regions[largest_idx];
            let food_per_spot = params.food_amount / params.num_food_spots.max(1) as i32;
            let num_spots = params.num_food_spots.min(walkable.len());

            for _ in 0..num_spots {
                let idx = rng.gen_range(0..walkable.len());
                let (fx, fy) = walkable[idx];
                let radius = rng.gen_range(2..=4);
                world.food_spawners.push(FoodSpawner {
                    x: fx,
                    y: fy,
                    radius,
                    amount: food_per_spot / 20,
                    interval: 5000,
                });
            }
        }

        world
    }

    /// Load a world from the JSON map format.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let map: MapJson =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {e}"))?;

        if map.width == 0 || map.height == 0 {
            return Err("World dimensions must be > 0".into());
        }

        let mut world = World::new(map.width, map.height);
        world.koth_x = map.koth_x.unwrap_or(map.width / 2);
        world.koth_y = map.koth_y.unwrap_or(map.height / 2);
        world.food_spawners = map.food_spawners.unwrap_or_default();

        for t in &map.tiles {
            if t.x >= map.width || t.y >= map.height {
                return Err(format!("Tile ({}, {}) out of bounds", t.x, t.y));
            }
            let idx = t.y * map.width + t.x;
            world.tiles[idx].tile_type = t.tile_type;
            world.tiles[idx].gfx = t.gfx.unwrap_or(if t.tile_type == TILE_PLAIN {
                TILE_GFX_PLAIN
            } else {
                TILE_GFX_SOLID
            });
        }

        Ok(world)
    }

    // --- Index helper ---

    #[inline]
    fn index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    // --- Tile queries ---

    /// Returns true if (x, y) is within the map bounds.
    pub fn is_on_map(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height
    }

    /// Returns true if (x, y) is strictly inside the outer border (x>=1, x<w-1, y>=1, y<h-1).
    pub fn is_within_border(&self, x: usize, y: usize) -> bool {
        x >= 1 && x < self.width.saturating_sub(1) && y >= 1 && y < self.height.saturating_sub(1)
    }

    /// Returns true if the tile at (tx, ty) is on the map and walkable (TILE_PLAIN).
    pub fn is_walkable(&self, tx: usize, ty: usize) -> bool {
        self.is_on_map(tx, ty) && self.tiles[self.index(tx, ty)].tile_type == TILE_PLAIN
    }

    /// Get the tile type at (x, y). Returns TILE_SOLID for out-of-bounds.
    pub fn get_type(&self, x: usize, y: usize) -> u8 {
        if !self.is_on_map(x, y) {
            return TILE_SOLID;
        }
        self.tiles[self.index(x, y)].tile_type
    }

    /// Get the tile gfx at (x, y). Returns TILE_GFX_SOLID for out-of-bounds.
    pub fn get_gfx(&self, x: usize, y: usize) -> u8 {
        if !self.is_on_map(x, y) {
            return TILE_GFX_SOLID;
        }
        self.tiles[self.index(x, y)].gfx
    }

    // --- Tile modifications ---

    /// Set the tile type. Only TILE_PLAIN is allowed, and only within the border.
    /// Returns true on success.
    pub fn set_type(&mut self, x: usize, y: usize, tile_type: u8) -> bool {
        if tile_type != TILE_PLAIN {
            return false;
        }
        if !self.is_within_border(x, y) {
            return false;
        }
        let idx = self.index(x, y);
        self.tiles[idx].tile_type = tile_type;
        self.tiles[idx].gfx = TILE_GFX_PLAIN;
        true
    }

    /// Set the tile gfx value. Returns true on success, false if out of bounds.
    pub fn set_gfx(&mut self, x: usize, y: usize, gfx: u8) -> bool {
        if !self.is_on_map(x, y) {
            return false;
        }
        let idx = self.index(x, y);
        self.tiles[idx].gfx = gfx;
        true
    }

    // --- Food ---

    /// Get the food at tile (x, y). Returns 0 for out-of-bounds.
    pub fn get_food(&self, x: usize, y: usize) -> i32 {
        if !self.is_on_map(x, y) {
            return 0;
        }
        self.tiles[self.index(x, y)].food
    }

    /// Add food to tile (x, y), clamping to [0, MAX_TILE_FOOD].
    /// Returns the actual change in food (can be negative if amount is negative).
    pub fn add_food(&mut self, x: usize, y: usize, amount: i32) -> i32 {
        if !self.is_on_map(x, y) {
            return 0;
        }
        let idx = self.index(x, y);
        let old = self.tiles[idx].food;
        let new_val = (old + amount).clamp(0, MAX_TILE_FOOD);
        self.tiles[idx].food = new_val;
        new_val - old
    }

    /// Eat food from tile (x, y). Returns the amount actually eaten (up to available food).
    pub fn eat_food(&mut self, x: usize, y: usize, amount: i32) -> i32 {
        if !self.is_on_map(x, y) || amount <= 0 {
            return 0;
        }
        let idx = self.index(x, y);
        let available = self.tiles[idx].food;
        let eaten = amount.min(available);
        self.tiles[idx].food -= eaten;
        eaten
    }

    // --- Utility ---

    /// Find a random walkable (TILE_PLAIN) tile. Returns None if no walkable tiles exist.
    pub fn find_plain_tile(&self) -> Option<(usize, usize)> {
        let walkable: Vec<(usize, usize)> = self
            .tiles
            .iter()
            .enumerate()
            .filter(|(_, t)| t.tile_type == TILE_PLAIN)
            .map(|(i, _)| (i % self.width, i / self.width))
            .collect();

        if walkable.is_empty() {
            return None;
        }

        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..walkable.len());
        Some(walkable[idx])
    }

    /// Returns the playable area in pixel coordinates: (x1, y1, x2, y2).
    /// The playable area is from border+1 to width-2 / height-2 tiles, converted to pixels.
    pub fn world_size_pixels(&self) -> (i32, i32, i32, i32) {
        let x1 = TILE_SIZE;
        let y1 = TILE_SIZE;
        let x2 = (self.width as i32 - 2) * TILE_SIZE;
        let y2 = (self.height as i32 - 2) * TILE_SIZE;
        (x1, y1, x2, y2)
    }

    /// Returns the King of the Hill tile position.
    pub fn koth_pos(&self) -> (usize, usize) {
        (self.koth_x, self.koth_y)
    }

    /// Returns the center of the King of the Hill tile in pixel coordinates.
    pub fn koth_center_pixels(&self) -> (i32, i32) {
        (
            Self::tile_center(self.koth_x),
            Self::tile_center(self.koth_y),
        )
    }

    // --- Coordinate conversions ---

    /// Convert a pixel coordinate to a tile coordinate.
    pub fn pixel_to_tile(px: i32) -> usize {
        (px / TILE_SIZE) as usize
    }

    /// Get the pixel coordinate of the center of a tile.
    pub fn tile_center(tx: usize) -> i32 {
        tx as i32 * TILE_SIZE + TILE_SIZE / 2
    }

    // --- Pathfinding (A* on tile grid) ---

    /// Find a path from (sx, sy) to (ex, ey) in pixel coordinates.
    /// Returns a list of waypoints (pixel coordinates of tile centers), or None if no path exists.
    pub fn find_path(&self, sx: i32, sy: i32, ex: i32, ey: i32) -> Option<Vec<(i32, i32)>> {
        let start_tx = Self::pixel_to_tile(sx);
        let start_ty = Self::pixel_to_tile(sy);
        let end_tx = Self::pixel_to_tile(ex);
        let end_ty = Self::pixel_to_tile(ey);

        if !self.is_walkable(start_tx, start_ty) || !self.is_walkable(end_tx, end_ty) {
            return None;
        }

        if start_tx == end_tx && start_ty == end_ty {
            return Some(vec![(Self::tile_center(end_tx), Self::tile_center(end_ty))]);
        }

        let w = self.width;
        let h = self.height;
        let size = w * h;

        let mut g_score = vec![i32::MAX; size];
        let mut came_from = vec![usize::MAX; size];
        let mut closed = vec![false; size];

        let start_idx = start_ty * w + start_tx;
        let end_idx = end_ty * w + end_tx;

        g_score[start_idx] = 0;

        let heuristic = |tx: usize, ty: usize| -> i32 {
            let dx = (tx as i32 - end_tx as i32).abs();
            let dy = (ty as i32 - end_ty as i32).abs();
            dx + dy // Manhattan distance
        };

        let mut open = BinaryHeap::new();
        open.push(AStarNode {
            cost: heuristic(start_tx, start_ty),
            x: start_tx,
            y: start_ty,
        });

        // 4-directional neighbors: right, left, down, up
        let dirs: [(i32, i32); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];

        while let Some(current) = open.pop() {
            let cx = current.x;
            let cy = current.y;
            let cidx = cy * w + cx;

            if cidx == end_idx {
                // Reconstruct path
                let mut path_tiles = Vec::new();
                let mut idx = end_idx;
                while idx != start_idx {
                    let ty = idx / w;
                    let tx = idx % w;
                    path_tiles.push((Self::tile_center(tx), Self::tile_center(ty)));
                    idx = came_from[idx];
                }
                path_tiles.reverse();
                return Some(path_tiles);
            }

            if closed[cidx] {
                continue;
            }
            closed[cidx] = true;

            let current_g = g_score[cidx];

            for (dx, dy) in &dirs {
                let nx = cx as i32 + dx;
                let ny = cy as i32 + dy;

                if nx < 0 || ny < 0 {
                    continue;
                }
                let nx = nx as usize;
                let ny = ny as usize;

                if nx >= w || ny >= h {
                    continue;
                }

                if !self.is_walkable(nx, ny) {
                    continue;
                }

                let nidx = ny * w + nx;
                if closed[nidx] {
                    continue;
                }

                let tentative_g = current_g + 1;
                if tentative_g < g_score[nidx] {
                    g_score[nidx] = tentative_g;
                    came_from[nidx] = cidx;
                    open.push(AStarNode {
                        cost: tentative_g + heuristic(nx, ny),
                        x: nx,
                        y: ny,
                    });
                }
            }
        }

        None // No path found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_world() {
        let w = World::new(10, 8);
        assert_eq!(w.width, 10);
        assert_eq!(w.height, 8);
        assert_eq!(w.tiles.len(), 80);
        // All tiles should be solid
        for tile in &w.tiles {
            assert_eq!(tile.tile_type, TILE_SOLID);
            assert_eq!(tile.food, 0);
        }
    }

    #[test]
    fn test_is_on_map() {
        let w = World::new(10, 8);
        assert!(w.is_on_map(0, 0));
        assert!(w.is_on_map(9, 7));
        assert!(!w.is_on_map(10, 0));
        assert!(!w.is_on_map(0, 8));
    }

    #[test]
    fn test_is_within_border() {
        let w = World::new(10, 8);
        assert!(!w.is_within_border(0, 0));
        assert!(!w.is_within_border(9, 7));
        assert!(!w.is_within_border(0, 4));
        assert!(!w.is_within_border(9, 4));
        assert!(w.is_within_border(1, 1));
        assert!(w.is_within_border(8, 6));
        assert!(!w.is_within_border(9, 6));
    }

    #[test]
    fn test_set_type() {
        let mut w = World::new(10, 8);

        // Can set TILE_PLAIN within border
        assert!(w.set_type(5, 4, TILE_PLAIN));
        assert_eq!(w.get_type(5, 4), TILE_PLAIN);

        // Cannot set TILE_SOLID (only TILE_PLAIN allowed)
        assert!(!w.set_type(5, 3, TILE_SOLID));

        // Cannot set on border
        assert!(!w.set_type(0, 0, TILE_PLAIN));
        assert!(!w.set_type(9, 7, TILE_PLAIN));

        // Cannot set out of bounds
        assert!(!w.set_type(10, 4, TILE_PLAIN));
    }

    #[test]
    fn test_set_gfx() {
        let mut w = World::new(10, 8);
        assert!(w.set_gfx(0, 0, TILE_GFX_BORDER));
        assert_eq!(w.get_gfx(0, 0), TILE_GFX_BORDER);
        assert!(!w.set_gfx(10, 0, TILE_GFX_PLAIN));
    }

    #[test]
    fn test_walkability() {
        let mut w = World::new(10, 8);
        assert!(!w.is_walkable(5, 4)); // solid by default
        w.set_type(5, 4, TILE_PLAIN);
        assert!(w.is_walkable(5, 4));
        assert!(!w.is_walkable(0, 0)); // border, still solid
        assert!(!w.is_walkable(20, 20)); // out of bounds
    }

    #[test]
    fn test_food_operations() {
        let mut w = World::new(10, 8);

        // Initially 0 food
        assert_eq!(w.get_food(5, 4), 0);

        // Add food
        let added = w.add_food(5, 4, 500);
        assert_eq!(added, 500);
        assert_eq!(w.get_food(5, 4), 500);

        // Add more - clamped at MAX_TILE_FOOD
        let added = w.add_food(5, 4, MAX_TILE_FOOD);
        assert_eq!(added, MAX_TILE_FOOD - 500);
        assert_eq!(w.get_food(5, 4), MAX_TILE_FOOD);

        // Negative add (remove)
        let added = w.add_food(5, 4, -100);
        assert_eq!(added, -100);
        assert_eq!(w.get_food(5, 4), MAX_TILE_FOOD - 100);

        // Eat food
        let eaten = w.eat_food(5, 4, 200);
        assert_eq!(eaten, 200);
        assert_eq!(w.get_food(5, 4), MAX_TILE_FOOD - 300);

        // Eat more than available
        let current = w.get_food(5, 4);
        let eaten = w.eat_food(5, 4, current + 1000);
        assert_eq!(eaten, current);
        assert_eq!(w.get_food(5, 4), 0);

        // Eat with 0 amount
        assert_eq!(w.eat_food(5, 4, 0), 0);

        // Out of bounds
        assert_eq!(w.get_food(20, 20), 0);
        assert_eq!(w.add_food(20, 20, 100), 0);
        assert_eq!(w.eat_food(20, 20, 100), 0);
    }

    #[test]
    fn test_food_clamping_at_zero() {
        let mut w = World::new(10, 8);
        w.add_food(5, 4, 100);
        // Remove more than available via add_food (negative)
        let change = w.add_food(5, 4, -500);
        assert_eq!(change, -100); // clamped at 0
        assert_eq!(w.get_food(5, 4), 0);
    }

    #[test]
    fn test_pixel_tile_conversion() {
        // pixel_to_tile
        assert_eq!(World::pixel_to_tile(0), 0);
        assert_eq!(World::pixel_to_tile(255), 0);
        assert_eq!(World::pixel_to_tile(256), 1);
        assert_eq!(World::pixel_to_tile(512), 2);

        // tile_center
        assert_eq!(World::tile_center(0), 128); // 0*256 + 128
        assert_eq!(World::tile_center(1), 384); // 1*256 + 128
        assert_eq!(World::tile_center(2), 640); // 2*256 + 128
    }

    #[test]
    fn test_world_size_pixels() {
        let w = World::new(10, 8);
        let (x1, y1, x2, y2) = w.world_size_pixels();
        assert_eq!(x1, TILE_SIZE); // 1 * 256
        assert_eq!(y1, TILE_SIZE);
        assert_eq!(x2, 8 * TILE_SIZE); // (10-2) * 256
        assert_eq!(y2, 6 * TILE_SIZE); // (8-2) * 256
    }

    #[test]
    fn test_koth() {
        let w = World::new(10, 8);
        assert_eq!(w.koth_pos(), (5, 4)); // defaults to center
        let (kx, ky) = w.koth_center_pixels();
        assert_eq!(kx, World::tile_center(5));
        assert_eq!(ky, World::tile_center(4));
    }

    #[test]
    fn test_find_plain_tile_none() {
        let w = World::new(10, 8);
        // All solid, no walkable tile
        assert!(w.find_plain_tile().is_none());
    }

    #[test]
    fn test_find_plain_tile_some() {
        let mut w = World::new(10, 8);
        w.set_type(5, 4, TILE_PLAIN);
        let result = w.find_plain_tile();
        assert_eq!(result, Some((5, 4))); // only one walkable tile
    }

    fn make_open_world(width: usize, height: usize) -> World {
        let mut w = World::new(width, height);
        for y in 1..height - 1 {
            for x in 1..width - 1 {
                w.set_type(x, y, TILE_PLAIN);
            }
        }
        w
    }

    #[test]
    fn test_pathfinding_simple() {
        let w = make_open_world(10, 10);
        let sx = World::tile_center(2);
        let sy = World::tile_center(2);
        let ex = World::tile_center(5);
        let ey = World::tile_center(2);
        let path = w.find_path(sx, sy, ex, ey);
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(!path.is_empty());
        // Last waypoint should be the destination tile center
        let last = path.last().unwrap();
        assert_eq!(*last, (World::tile_center(5), World::tile_center(2)));
    }

    #[test]
    fn test_pathfinding_same_tile() {
        let w = make_open_world(10, 10);
        let cx = World::tile_center(3);
        let cy = World::tile_center(3);
        let path = w.find_path(cx, cy, cx, cy);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 1);
    }

    #[test]
    fn test_pathfinding_blocked() {
        let w = World::new(10, 10); // all solid
        let sx = World::tile_center(2);
        let sy = World::tile_center(2);
        let ex = World::tile_center(5);
        let ey = World::tile_center(5);
        assert!(w.find_path(sx, sy, ex, ey).is_none());
    }

    #[test]
    fn test_pathfinding_around_wall() {
        let mut w = make_open_world(10, 10);
        // Create a wall across the middle (column 5, rows 1-6)
        // We need to set tiles back to solid. We can't use set_type for SOLID,
        // so we manipulate directly.
        for y in 1..7 {
            let idx = y * w.width + 5;
            w.tiles[idx].tile_type = TILE_SOLID;
        }
        // Path from (3,4) to (7,4) should go around the wall
        let sx = World::tile_center(3);
        let sy = World::tile_center(4);
        let ex = World::tile_center(7);
        let ey = World::tile_center(4);
        let path = w.find_path(sx, sy, ex, ey);
        assert!(path.is_some());
        let path = path.unwrap();
        // Path should not go through column 5, rows 1-6
        for &(px, py) in &path {
            let tx = World::pixel_to_tile(px);
            let ty = World::pixel_to_tile(py);
            if tx == 5 && ty < 7 {
                panic!("Path went through wall at tile ({}, {})", tx, ty);
            }
        }
    }

    #[test]
    fn test_pathfinding_unreachable() {
        let mut w = make_open_world(10, 10);
        // Create a complete wall isolating right side
        for y in 1..9 {
            let idx = y * w.width + 5;
            w.tiles[idx].tile_type = TILE_SOLID;
        }
        let sx = World::tile_center(2);
        let sy = World::tile_center(4);
        let ex = World::tile_center(8);
        let ey = World::tile_center(4);
        assert!(w.find_path(sx, sy, ex, ey).is_none());
    }

    #[test]
    fn test_from_json() {
        let json = r#"{
            "name": "test",
            "width": 10,
            "height": 8,
            "koth_x": 5,
            "koth_y": 4,
            "tiles": [
                {"x": 3, "y": 3, "type": 1, "gfx": 1},
                {"x": 4, "y": 3, "type": 1, "gfx": 1},
                {"x": 5, "y": 3, "type": 1}
            ],
            "food_spawners": [
                {"x": 4, "y": 3, "radius": 2, "amount": 100, "interval": 50}
            ]
        }"#;
        let w = World::from_json(json).unwrap();
        assert_eq!(w.width, 10);
        assert_eq!(w.height, 8);
        assert_eq!(w.koth_x, 5);
        assert_eq!(w.koth_y, 4);
        assert_eq!(w.get_type(3, 3), TILE_PLAIN);
        assert_eq!(w.get_type(4, 3), TILE_PLAIN);
        assert_eq!(w.get_type(5, 3), TILE_PLAIN);
        assert_eq!(w.get_gfx(5, 3), TILE_GFX_PLAIN); // default gfx for TILE_PLAIN
        assert_eq!(w.get_type(0, 0), TILE_SOLID);
        assert_eq!(w.food_spawners.len(), 1);
        assert_eq!(w.food_spawners[0].amount, 100);
    }

    #[test]
    fn test_from_json_invalid() {
        assert!(World::from_json("not json").is_err());
        // Out of bounds tile
        let json = r#"{"width": 5, "height": 5, "tiles": [{"x": 10, "y": 0, "type": 1}]}"#;
        assert!(World::from_json(json).is_err());
        // Zero dimensions
        let json = r#"{"width": 0, "height": 5, "tiles": []}"#;
        assert!(World::from_json(json).is_err());
    }

    #[test]
    fn test_from_json_defaults() {
        let json = r#"{"width": 10, "height": 8, "tiles": []}"#;
        let w = World::from_json(json).unwrap();
        assert_eq!(w.koth_x, 5); // defaults to center
        assert_eq!(w.koth_y, 4);
        assert!(w.food_spawners.is_empty());
    }

    #[test]
    fn test_generate_map_2_players() {
        let w = World::generate_map(2);
        assert_eq!(w.width, 30);
        assert_eq!(w.height, 30);
        // Border should be solid
        assert!(!w.is_walkable(0, 0));
        assert!(!w.is_walkable(29, 29));
        // Should have walkable tiles
        assert!(w.find_plain_tile().is_some());
        // Should have food spawners (base + 2 spawn-point spawners)
        assert!(w.food_spawners.len() >= 10);
        // KOTH should be on a walkable tile
        assert!(w.is_walkable(w.koth_x, w.koth_y));
    }

    #[test]
    fn test_generate_map_4_players() {
        let w = World::generate_map(4);
        assert_eq!(w.width, 50);
        assert_eq!(w.height, 50);
        assert!(w.find_plain_tile().is_some());
        assert!(w.is_walkable(w.koth_x, w.koth_y));
        // Should have more food spawners for more players
        assert!(w.food_spawners.len() >= 14);
    }

    #[test]
    fn test_generate_map_8_players() {
        let w = World::generate_map(8);
        assert_eq!(w.width, 80);
        assert_eq!(w.height, 80);
        assert!(w.find_plain_tile().is_some());
        assert!(w.is_walkable(w.koth_x, w.koth_y));
    }

    #[test]
    fn test_generate_map_16_players() {
        let w = World::generate_map(16);
        assert_eq!(w.width, 100);
        assert_eq!(w.height, 100);
        assert!(w.find_plain_tile().is_some());
    }

    #[test]
    fn test_generate_map_large_player_count() {
        let w = World::generate_map(20);
        // 100 + (20-16)*2 = 108
        assert_eq!(w.width, 108);
        assert_eq!(w.height, 108);
        assert!(w.find_plain_tile().is_some());
    }
}
