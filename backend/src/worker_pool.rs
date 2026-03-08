// Worker pool for parallel headless game execution.
//
// Each worker runs on a dedicated OS thread (Game is !Send due to Rc<RefCell<>>).
// The pool has a fixed capacity; callers check `has_capacity()` before dispatching.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::engine::server::{run_game_headless, GameResult, PlayerEntry};
use crate::engine::world::World;
use crate::metrics;

/// Manages a fixed-size pool of OS threads for headless game execution.
pub struct WorkerPool {
    worker_count: usize,
    active_workers: Arc<AtomicUsize>,
}

impl WorkerPool {
    pub fn new(worker_count: usize) -> Self {
        Self {
            worker_count,
            active_workers: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Whether the pool has capacity to accept another game.
    pub fn has_capacity(&self) -> bool {
        self.active_workers.load(Ordering::Relaxed) < self.worker_count
    }

    /// Current number of active workers.
    pub fn active_count(&self) -> usize {
        self.active_workers.load(Ordering::Relaxed)
    }

    /// Spawn a headless game on a new OS thread.
    /// Returns false if the pool is at capacity.
    /// The `on_complete` callback is invoked asynchronously via the tokio runtime
    /// after the game finishes.
    pub fn spawn_game<F>(
        &self,
        world: World,
        players: Vec<PlayerEntry>,
        max_ticks: u64,
        match_id: Option<i64>,
        bot_version_ids: Vec<i64>,
        on_complete: F,
    ) -> bool
    where
        F: FnOnce(GameResult) + Send + 'static,
    {
        if !self.has_capacity() {
            return false;
        }

        let active = self.active_workers.clone();
        active.fetch_add(1, Ordering::Relaxed);
        metrics::HEADLESS_WORKERS_ACTIVE.set(active.load(Ordering::Relaxed) as i64);

        let thread_name = format!("headless-game-{}", match_id.unwrap_or(0));

        let rt_handle = tokio::runtime::Handle::current();

        std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || {
                let result =
                    run_game_headless(world, players, max_ticks, match_id, bot_version_ids);

                // Decrement active count
                active.fetch_sub(1, Ordering::Relaxed);
                metrics::HEADLESS_WORKERS_ACTIVE.set(active.load(Ordering::Relaxed) as i64);

                // Run async completion logic on the tokio runtime
                rt_handle.spawn(async move {
                    on_complete(result);
                });
            })
            .expect("failed to spawn headless game thread");

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_pool_capacity() {
        let pool = WorkerPool::new(4);
        assert!(pool.has_capacity());
        assert_eq!(pool.active_count(), 0);
    }

    #[test]
    fn test_worker_pool_zero_capacity() {
        let pool = WorkerPool::new(0);
        assert!(!pool.has_capacity());
    }
}
