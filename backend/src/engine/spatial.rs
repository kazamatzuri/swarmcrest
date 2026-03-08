/// Grid-based spatial index for efficient creature proximity queries.
///
/// Divides the world into cells and tracks which creatures are in each cell.
/// This reduces nearest-enemy queries from O(n^2) to O(n * k) where k is
/// the number of creatures in nearby cells.
use super::config::TILE_SIZE;

/// Size of each spatial grid cell in pixels. Using 2 tiles (512px) as cell size
/// gives a good balance between granularity and overhead.
const CELL_SIZE: i32 = TILE_SIZE * 2; // 512 pixels per cell

/// Entry in the spatial index: creature ID + position + player ID.
#[derive(Clone, Debug)]
pub struct SpatialEntry {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub player_id: u32,
}

/// A grid-based spatial index. Each cell contains a list of creature entries.
pub struct SpatialGrid {
    /// Number of cells in X direction.
    pub cols: usize,
    /// Number of cells in Y direction.
    pub rows: usize,
    /// Flat array of cells, indexed by row * cols + col.
    cells: Vec<Vec<SpatialEntry>>,
}

impl SpatialGrid {
    /// Create a new spatial grid for a world of the given pixel dimensions.
    pub fn new(world_width_tiles: usize, world_height_tiles: usize) -> Self {
        let world_px_w = world_width_tiles as i32 * TILE_SIZE;
        let world_px_h = world_height_tiles as i32 * TILE_SIZE;
        let cols = ((world_px_w + CELL_SIZE - 1) / CELL_SIZE).max(1) as usize;
        let rows = ((world_px_h + CELL_SIZE - 1) / CELL_SIZE).max(1) as usize;
        SpatialGrid {
            cols,
            rows,
            cells: vec![Vec::new(); cols * rows],
        }
    }

    /// Clear all entries from the grid.
    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.clear();
        }
    }

    /// Insert a creature into the grid.
    pub fn insert(&mut self, id: u32, x: i32, y: i32, player_id: u32) {
        let (col, row) = self.cell_coords(x, y);
        let idx = row * self.cols + col;
        self.cells[idx].push(SpatialEntry {
            id,
            x,
            y,
            player_id,
        });
    }

    /// Find the nearest enemy creature to the given position.
    /// Returns (id, x, y, player_id, distance) or None if no enemies exist.
    pub fn find_nearest_enemy(
        &self,
        x: i32,
        y: i32,
        my_player_id: u32,
    ) -> Option<(u32, i32, i32, u32, i32)> {
        let (cx, cy) = self.cell_coords(x, y);

        let mut best: Option<(u32, i32, i32, u32, i64)> = None;
        let mut best_dist_sq: i64 = i64::MAX;

        // Search outward in rings. Start with the creature's cell, then expand.
        // We can stop expanding once the minimum possible distance from the next ring
        // exceeds our current best distance.
        let max_ring = self.cols.max(self.rows);

        for ring in 0..max_ring {
            // Minimum distance from center of current cell to edge of this ring (in pixels)
            let ring_min_dist = if ring == 0 {
                0i64
            } else {
                (ring as i64 - 1) * CELL_SIZE as i64
            };
            let ring_min_dist_sq = ring_min_dist * ring_min_dist;

            // If we already have a candidate closer than the nearest possible point in this ring,
            // we can stop.
            if ring > 0 && ring_min_dist_sq > best_dist_sq {
                break;
            }

            // Iterate over cells in this ring
            let r = ring as i32;
            for dy in -r..=r {
                for dx in -r..=r {
                    // Only process cells on the border of this ring (not interior, already done)
                    if ring > 0 && dx.abs() < r && dy.abs() < r {
                        continue;
                    }

                    let col = cx as i32 + dx;
                    let row = cy as i32 + dy;
                    if col < 0 || row < 0 || col >= self.cols as i32 || row >= self.rows as i32 {
                        continue;
                    }

                    let idx = row as usize * self.cols + col as usize;
                    for entry in &self.cells[idx] {
                        if entry.player_id == my_player_id {
                            continue;
                        }
                        let edx = (x - entry.x) as i64;
                        let edy = (y - entry.y) as i64;
                        let dist_sq = edx * edx + edy * edy;
                        if dist_sq < best_dist_sq {
                            best_dist_sq = dist_sq;
                            best = Some((entry.id, entry.x, entry.y, entry.player_id, dist_sq));
                        }
                    }
                }
            }
        }

        best.map(|(id, ex, ey, pid, dist_sq)| {
            let dist = (dist_sq as f64).sqrt() as i32;
            (id, ex, ey, pid, dist)
        })
    }

    /// Get all creatures in the given cell and its neighbors (3x3 area).
    /// Useful for collision queries or area-of-effect lookups.
    pub fn query_neighborhood(&self, x: i32, y: i32) -> Vec<&SpatialEntry> {
        let (cx, cy) = self.cell_coords(x, y);
        let mut results = Vec::new();

        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let col = cx as i32 + dx;
                let row = cy as i32 + dy;
                if col >= 0 && row >= 0 && col < self.cols as i32 && row < self.rows as i32 {
                    let idx = row as usize * self.cols + col as usize;
                    for entry in &self.cells[idx] {
                        results.push(entry);
                    }
                }
            }
        }
        results
    }

    /// Convert pixel coordinates to cell coordinates, clamped to grid bounds.
    fn cell_coords(&self, x: i32, y: i32) -> (usize, usize) {
        let col = (x / CELL_SIZE).clamp(0, self.cols as i32 - 1) as usize;
        let row = (y / CELL_SIZE).clamp(0, self.rows as i32 - 1) as usize;
        (col, row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_grid() {
        let grid = SpatialGrid::new(10, 10);
        // 10 tiles * 256px = 2560px, / 512 cell_size = 5 cells
        assert_eq!(grid.cols, 5);
        assert_eq!(grid.rows, 5);
        assert_eq!(grid.cells.len(), 25);
    }

    #[test]
    fn test_insert_and_query() {
        let mut grid = SpatialGrid::new(10, 10);
        grid.insert(1, 128, 128, 1); // cell (0,0)
        grid.insert(2, 600, 128, 2); // cell (1,0)

        let neighbors = grid.query_neighborhood(128, 128);
        // Should find both since cell (1,0) is adjacent to (0,0)
        assert_eq!(neighbors.len(), 2);
    }

    #[test]
    fn test_find_nearest_enemy_basic() {
        let mut grid = SpatialGrid::new(20, 20);
        // Player 1 creature at (500, 500)
        grid.insert(1, 500, 500, 1);
        // Player 2 creatures at varying distances
        grid.insert(10, 600, 500, 2); // dist 100
        grid.insert(11, 1000, 500, 2); // dist 500
        grid.insert(12, 2000, 2000, 2); // far away

        let result = grid.find_nearest_enemy(500, 500, 1);
        assert!(result.is_some());
        let (id, _x, _y, _pid, dist) = result.unwrap();
        assert_eq!(id, 10);
        assert_eq!(dist, 100);
    }

    #[test]
    fn test_find_nearest_enemy_no_enemies() {
        let mut grid = SpatialGrid::new(10, 10);
        grid.insert(1, 500, 500, 1);
        grid.insert(2, 600, 500, 1); // same player

        let result = grid.find_nearest_enemy(500, 500, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_nearest_enemy_empty() {
        let grid = SpatialGrid::new(10, 10);
        let result = grid.find_nearest_enemy(500, 500, 1);
        assert!(result.is_none());
    }

    #[test]
    fn test_clear() {
        let mut grid = SpatialGrid::new(10, 10);
        grid.insert(1, 128, 128, 1);
        grid.insert(2, 600, 128, 2);
        grid.clear();

        let neighbors = grid.query_neighborhood(128, 128);
        assert_eq!(neighbors.len(), 0);
    }

    #[test]
    fn test_cell_coords_clamping() {
        let grid = SpatialGrid::new(10, 10);
        // Negative coords should clamp to (0,0)
        let (col, row) = grid.cell_coords(-100, -100);
        assert_eq!(col, 0);
        assert_eq!(row, 0);

        // Very large coords should clamp to max
        let (col, row) = grid.cell_coords(999999, 999999);
        assert_eq!(col, grid.cols - 1);
        assert_eq!(row, grid.rows - 1);
    }

    #[test]
    fn test_find_nearest_enemy_ring_expansion() {
        // Test that the ring-based search correctly finds enemies far away
        let mut grid = SpatialGrid::new(40, 40);
        grid.insert(1, 100, 100, 1);
        // Place an enemy far away (should be found via ring expansion)
        grid.insert(2, 5000, 5000, 2);

        let result = grid.find_nearest_enemy(100, 100, 1);
        assert!(result.is_some());
        let (id, _, _, _, _) = result.unwrap();
        assert_eq!(id, 2);
    }

    #[test]
    fn test_find_nearest_picks_closest() {
        let mut grid = SpatialGrid::new(20, 20);
        // Place enemies at different distances
        grid.insert(10, 200, 0, 2); // dist 200
        grid.insert(11, 0, 300, 2); // dist 300
        grid.insert(12, 100, 0, 2); // dist 100 (closest)

        let result = grid.find_nearest_enemy(0, 0, 1);
        assert!(result.is_some());
        let (id, _, _, _, dist) = result.unwrap();
        assert_eq!(id, 12);
        assert_eq!(dist, 100);
    }
}
