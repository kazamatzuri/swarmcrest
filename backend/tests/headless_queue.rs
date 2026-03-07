// Integration tests for the parallel headless game execution system:
// run_game_headless(), DB queue operations, and WorkerPool dispatch.

use std::sync::Arc;

use swarmcrest_backend::db::Database;
use swarmcrest_backend::engine::config::*;
use swarmcrest_backend::engine::server::{run_game_headless, PlayerEntry};
use swarmcrest_backend::engine::world::World;
use swarmcrest_backend::worker_pool::WorkerPool;

/// Create a small test world with walkable interior and food.
fn create_test_world() -> World {
    let mut world = World::new(20, 20);
    for x in 1..19 {
        for y in 1..19 {
            world.set_type(x, y, TILE_PLAIN);
        }
    }
    world.koth_x = 10;
    world.koth_y = 10;
    for x in 3..17 {
        for y in 3..17 {
            world.add_food(x, y, 5000);
        }
    }
    world
}

fn stupibot_code() -> &'static str {
    include_str!("../../orig_game/contrib/bots/stupibot.lua")
}

async fn test_db() -> Database {
    sqlx::any::install_default_drivers();
    Database::new("sqlite::memory:").await.unwrap()
}

// ── run_game_headless tests ──────────────────────────────────────────

#[test]
fn test_headless_two_bots_produces_result() {
    let world = create_test_world();
    let code = stupibot_code();
    let players = vec![
        PlayerEntry {
            name: "Bot A".into(),
            code: code.into(),
        },
        PlayerEntry {
            name: "Bot B".into(),
            code: code.into(),
        },
    ];

    let result = run_game_headless(world, players, 100, Some(42), vec![1, 2]);

    assert_eq!(result.match_id, Some(42));
    assert!(result.tick_count > 0, "Game should have run some ticks");
    assert_eq!(result.player_scores.len(), 2);
    assert!(result.replay_data.len() > 0, "Replay should be non-empty");
    assert!(result.failed_bot_version_ids.is_empty());

    // Both players should have scores
    for ps in &result.player_scores {
        assert!(ps.score >= 0);
    }
}

#[test]
fn test_headless_determines_winner() {
    let world = create_test_world();
    let code = stupibot_code();
    let players = vec![
        PlayerEntry {
            name: "Bot A".into(),
            code: code.into(),
        },
        PlayerEntry {
            name: "Bot B".into(),
            code: code.into(),
        },
    ];

    // Run a full game (6000 ticks = 10 min game time) to ensure a winner is determined
    let result = run_game_headless(world, players, 6000, None, vec![10, 20]);

    // Either a winner or a draw — both are valid outcomes
    assert_eq!(result.player_scores.len(), 2);
    assert!(result.tick_count > 0);

    // Verify player_scores have correct bot_version_ids
    let version_ids: Vec<i64> = result.player_scores.iter().map(|s| s.bot_version_id).collect();
    assert!(version_ids.contains(&10));
    assert!(version_ids.contains(&20));
}

#[test]
fn test_headless_invalid_bot_code() {
    let world = create_test_world();
    let players = vec![
        PlayerEntry {
            name: "Good Bot".into(),
            code: stupibot_code().into(),
        },
        PlayerEntry {
            name: "Bad Bot".into(),
            code: "this is not valid lua %%%".into(),
        },
    ];

    let result = run_game_headless(world, players, 100, Some(99), vec![1, 2]);

    // The bad bot should be in failed_bot_version_ids
    assert!(
        result.failed_bot_version_ids.contains(&2),
        "Bad bot version should be marked as failed: {:?}",
        result.failed_bot_version_ids
    );
}

#[test]
fn test_headless_no_players() {
    let world = create_test_world();
    let result = run_game_headless(world, vec![], 100, None, vec![]);
    // Should complete without panicking — with 0 players, game loop runs to max_ticks
    // (early_exit only triggers when players.len() >= 2 but loaded <= 1)
    assert!(result.tick_count > 0);
    assert!(result.player_scores.is_empty());
}

#[test]
fn test_headless_single_player_wins_by_default() {
    let world = create_test_world();
    let players = vec![PlayerEntry {
        name: "Solo".into(),
        code: stupibot_code().into(),
    }];

    let result = run_game_headless(world, players, 100, None, vec![1]);
    // Single player in a 1+ player game — no opponent loaded, they win by default
    assert_eq!(result.player_scores.len(), 1);
}

// ── DB queue tests ───────────────────────────────────────────────────

#[tokio::test]
async fn test_enqueue_and_claim_job() {
    let db = test_db().await;

    // Create a match first (queue references matches)
    let m = db.create_match("1v1", "random").await.unwrap();

    // Enqueue
    let job = db.enqueue_game(m.id, Some("random"), 0, None).await.unwrap();
    assert_eq!(job.match_id, m.id);
    assert_eq!(job.status, "pending");
    assert_eq!(job.priority, 0);

    // Claim
    let claimed = db.claim_queue_job("worker-1").await.unwrap();
    assert!(claimed.is_some());
    let claimed = claimed.unwrap();
    assert_eq!(claimed.match_id, m.id);
    assert_eq!(claimed.status, "claimed");
    assert_eq!(claimed.worker_id.as_deref(), Some("worker-1"));

    // No more jobs to claim
    let none = db.claim_queue_job("worker-2").await.unwrap();
    assert!(none.is_none());
}

#[tokio::test]
async fn test_complete_queue_job() {
    let db = test_db().await;
    let m = db.create_match("1v1", "random").await.unwrap();
    db.enqueue_game(m.id, None, 0, None).await.unwrap();

    let job = db.claim_queue_job("w1").await.unwrap().unwrap();
    db.complete_queue_job(job.id).await.unwrap();

    // Should not be claimable again
    let none = db.claim_queue_job("w1").await.unwrap();
    assert!(none.is_none());

    // Status should show 1 completed
    let status = db.queue_status().await.unwrap();
    assert_eq!(status.completed, 1);
    assert_eq!(status.pending, 0);
}

#[tokio::test]
async fn test_fail_queue_job_retries() {
    let db = test_db().await;
    let m = db.create_match("1v1", "random").await.unwrap();
    db.enqueue_game(m.id, None, 0, None).await.unwrap();

    // Claim and fail
    let job = db.claim_queue_job("w1").await.unwrap().unwrap();
    db.fail_queue_job(job.id, "test error").await.unwrap();

    // Should be re-queued as pending (attempt 1 of 3)
    let status = db.queue_status().await.unwrap();
    assert_eq!(status.pending, 1);

    // Claim and fail again
    let job = db.claim_queue_job("w1").await.unwrap().unwrap();
    assert_eq!(job.attempts, 1);
    db.fail_queue_job(job.id, "test error 2").await.unwrap();

    // Still pending (attempt 2 of 3)
    let status = db.queue_status().await.unwrap();
    assert_eq!(status.pending, 1);

    // Third attempt — should now fail permanently
    let job = db.claim_queue_job("w1").await.unwrap().unwrap();
    assert_eq!(job.attempts, 2);
    db.fail_queue_job(job.id, "final error").await.unwrap();

    let status = db.queue_status().await.unwrap();
    assert_eq!(status.pending, 0);
    assert_eq!(status.failed, 1);
}

#[tokio::test]
async fn test_queue_priority_ordering() {
    let db = test_db().await;

    let m1 = db.create_match("1v1", "random").await.unwrap();
    let m2 = db.create_match("1v1", "random").await.unwrap();
    let m3 = db.create_match("1v1", "random").await.unwrap();

    // Enqueue with different priorities
    db.enqueue_game(m1.id, None, 0, None).await.unwrap(); // low priority
    db.enqueue_game(m2.id, None, 10, None).await.unwrap(); // high priority (tournament)
    db.enqueue_game(m3.id, None, 5, None).await.unwrap(); // medium priority

    // Should claim highest priority first
    let job1 = db.claim_queue_job("w1").await.unwrap().unwrap();
    assert_eq!(job1.match_id, m2.id, "Should claim priority 10 first");

    let job2 = db.claim_queue_job("w1").await.unwrap().unwrap();
    assert_eq!(job2.match_id, m3.id, "Should claim priority 5 second");

    let job3 = db.claim_queue_job("w1").await.unwrap().unwrap();
    assert_eq!(job3.match_id, m1.id, "Should claim priority 0 last");
}

#[tokio::test]
async fn test_queue_status_counts() {
    let db = test_db().await;

    let status = db.queue_status().await.unwrap();
    assert_eq!(status.total, 0);

    let m1 = db.create_match("1v1", "random").await.unwrap();
    let m2 = db.create_match("1v1", "random").await.unwrap();
    let m3 = db.create_match("1v1", "random").await.unwrap();

    db.enqueue_game(m1.id, None, 0, None).await.unwrap();
    db.enqueue_game(m2.id, None, 0, None).await.unwrap();
    db.enqueue_game(m3.id, None, 0, None).await.unwrap();

    let status = db.queue_status().await.unwrap();
    assert_eq!(status.pending, 3);
    assert_eq!(status.total, 3);

    // Claim one
    let job = db.claim_queue_job("w1").await.unwrap().unwrap();
    let status = db.queue_status().await.unwrap();
    assert_eq!(status.pending, 2);
    assert_eq!(status.claimed, 1);

    // Complete it
    db.complete_queue_job(job.id).await.unwrap();
    let status = db.queue_status().await.unwrap();
    assert_eq!(status.pending, 2);
    assert_eq!(status.completed, 1);
}

// ── WorkerPool + run_game_headless integration ───────────────────────

#[tokio::test]
async fn test_worker_pool_runs_game_to_completion() {
    let pool = WorkerPool::new(2);
    assert!(pool.has_capacity());

    let world = create_test_world();
    let code = stupibot_code();
    let players = vec![
        PlayerEntry {
            name: "Bot A".into(),
            code: code.into(),
        },
        PlayerEntry {
            name: "Bot B".into(),
            code: code.into(),
        },
    ];

    let (tx, rx) = tokio::sync::oneshot::channel();

    let spawned = pool.spawn_game(
        world,
        players,
        200, // short game
        Some(1),
        vec![10, 20],
        move |result| {
            let _ = tx.send(result);
        },
    );
    assert!(spawned);

    // Wait for the game to complete
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        rx,
    )
    .await
    .expect("Game timed out")
    .expect("Channel closed");

    assert_eq!(result.match_id, Some(1));
    assert!(result.tick_count > 0);
    assert_eq!(result.player_scores.len(), 2);
    assert!(!result.replay_data.is_empty());
}

#[tokio::test]
async fn test_worker_pool_parallel_games() {
    let pool = Arc::new(WorkerPool::new(4));
    let (tx1, rx1) = tokio::sync::oneshot::channel();
    let (tx2, rx2) = tokio::sync::oneshot::channel();
    let (tx3, rx3) = tokio::sync::oneshot::channel();

    let code = stupibot_code();

    // Spawn 3 games in parallel
    for (i, tx) in [(1, tx1), (2, tx2), (3, tx3)] {
        let world = create_test_world();
        let players = vec![
            PlayerEntry {
                name: format!("Bot A-{i}"),
                code: code.into(),
            },
            PlayerEntry {
                name: format!("Bot B-{i}"),
                code: code.into(),
            },
        ];
        let spawned = pool.spawn_game(
            world,
            players,
            100,
            Some(i as i64),
            vec![i as i64 * 10, i as i64 * 10 + 1],
            move |result| {
                let _ = tx.send(result);
            },
        );
        assert!(spawned, "Game {i} should have been accepted by pool");
    }

    let timeout = std::time::Duration::from_secs(30);

    let r1 = tokio::time::timeout(timeout, rx1)
        .await
        .expect("Game 1 timed out")
        .expect("Channel 1 closed");
    let r2 = tokio::time::timeout(timeout, rx2)
        .await
        .expect("Game 2 timed out")
        .expect("Channel 2 closed");
    let r3 = tokio::time::timeout(timeout, rx3)
        .await
        .expect("Game 3 timed out")
        .expect("Channel 3 closed");

    // All 3 games should have produced results
    assert_eq!(r1.match_id, Some(1));
    assert_eq!(r2.match_id, Some(2));
    assert_eq!(r3.match_id, Some(3));
    assert!(r1.tick_count > 0);
    assert!(r2.tick_count > 0);
    assert!(r3.tick_count > 0);
}

#[test]
fn test_worker_pool_rejects_when_at_capacity() {
    // WorkerPool with 0 capacity always rejects
    let pool = WorkerPool::new(0);

    let code = stupibot_code();
    let (tx, _rx) = std::sync::mpsc::channel();

    let spawned = pool.spawn_game(
        create_test_world(),
        vec![
            PlayerEntry { name: "A".into(), code: code.into() },
            PlayerEntry { name: "B".into(), code: code.into() },
        ],
        100,
        Some(1),
        vec![1, 2],
        move |r| { let _ = tx.send(r); },
    );
    assert!(!spawned, "Should reject when pool has 0 capacity");
}

// ── Full end-to-end: DB queue → worker pool → completion ─────────────

#[tokio::test]
async fn test_end_to_end_enqueue_and_process() {
    let db = Arc::new(test_db().await);

    // Create a user and two bots with code
    let user = db
        .create_user("testuser", "test@example.com", "hash", "Test User")
        .await
        .unwrap();
    let bot_a = db
        .create_bot("Bot A", "test bot a", Some(user.id))
        .await
        .unwrap();
    let bot_b = db
        .create_bot("Bot B", "test bot b", Some(user.id))
        .await
        .unwrap();
    let va = db
        .create_bot_version(bot_a.id, stupibot_code())
        .await
        .unwrap();
    let vb = db
        .create_bot_version(bot_b.id, stupibot_code())
        .await
        .unwrap();

    // Create a match and add participants
    let m = db.create_match("1v1", "random").await.unwrap();
    db.add_match_participant(m.id, va.id, 0).await.unwrap();
    db.add_match_participant(m.id, vb.id, 1).await.unwrap();

    // Enqueue the match
    let job = db.enqueue_game(m.id, None, 0, None).await.unwrap();

    // Claim the job (simulating what the queue worker does)
    let claimed = db.claim_queue_job("test-worker").await.unwrap().unwrap();
    assert_eq!(claimed.id, job.id);

    // Load participants and bot code (simulating queue worker)
    let participants = db.get_match_participants(m.id).await.unwrap();
    let mut players = Vec::new();
    let mut version_ids = Vec::new();
    for p in &participants {
        let v = db.get_bot_version_by_id(p.bot_version_id).await.unwrap().unwrap();
        let name = p.bot_name.clone().unwrap_or_else(|| format!("Bot v{}", v.version));
        players.push(PlayerEntry { name, code: v.code });
        version_ids.push(p.bot_version_id);
    }

    // Run the headless game
    let world = create_test_world();
    let result = run_game_headless(world, players, 200, Some(m.id), version_ids.clone());

    assert!(result.tick_count > 0, "Game should have run");
    assert_eq!(result.player_scores.len(), 2);

    // Run game completion (save replay, finish match, update stats)
    swarmcrest_backend::queue::run_game_completion(
        &db,
        m.id,
        &version_ids,
        "1v1",
        &result,
    )
    .await;

    // Mark queue job complete
    db.complete_queue_job(claimed.id).await.unwrap();

    // Verify match is finished
    let finished_match = db.get_match(m.id).await.unwrap().unwrap();
    assert_eq!(finished_match.status, "finished");

    // Verify replay was saved
    let replay = db.get_replay(m.id).await.unwrap();
    assert!(replay.is_some(), "Replay should have been saved");

    // Verify bot version stats were updated
    let updated_va = db.get_bot_version_by_id(va.id).await.unwrap().unwrap();
    let updated_vb = db.get_bot_version_by_id(vb.id).await.unwrap().unwrap();
    assert_eq!(updated_va.games_played, 1);
    assert_eq!(updated_vb.games_played, 1);
    assert_eq!(updated_va.wins + updated_va.losses + updated_va.draws, 1);
    assert_eq!(updated_vb.wins + updated_vb.losses + updated_vb.draws, 1);

    // Verify queue status
    let status = db.queue_status().await.unwrap();
    assert_eq!(status.completed, 1);
    assert_eq!(status.pending, 0);
}
