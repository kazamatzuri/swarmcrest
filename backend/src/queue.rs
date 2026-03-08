// DB-backed game queue with worker pool dispatch.
//
// Headless games are dispatched to the WorkerPool for parallel execution.
// Live (non-headless) games still use the single GameServer with WebSocket broadcast.

use std::path::PathBuf;
use std::sync::Arc;

use crate::api::resolve_map;
use crate::db::Database;
use crate::engine::server::PlayerEntry;
use crate::metrics;
use crate::worker_pool::WorkerPool;

/// Spawn a background task that polls the DB queue and dispatches games.
///
/// Headless games go to the `WorkerPool` for parallel execution.
pub fn spawn_queue_worker(
    db: Arc<Database>,
    worker_pool: Arc<WorkerPool>,
    maps_dir: PathBuf,
    poll_interval_ms: u64,
    worker_id: String,
) {
    tokio::spawn(async move {
        let poll_duration = tokio::time::Duration::from_millis(poll_interval_ms);

        loop {
            tokio::time::sleep(poll_duration).await;

            // Don't claim if no capacity
            if !worker_pool.has_capacity() {
                continue;
            }

            // Try to claim a job from the DB queue
            let job = match db.claim_queue_job(&worker_id).await {
                Ok(Some(job)) => job,
                Ok(None) => continue,
                Err(e) => {
                    tracing::error!("Queue worker: failed to claim job: {e}");
                    continue;
                }
            };

            tracing::info!(
                match_id = job.match_id,
                job_id = job.id,
                priority = job.priority,
                "Claimed queue job"
            );

            // Load match participants to get bot version IDs and names
            let participants = match db.get_match_participants(job.match_id).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(
                        "Queue worker: failed to load participants for match {}: {e}",
                        job.match_id
                    );
                    let _ = db
                        .fail_queue_job(job.id, &format!("Failed to load participants: {e}"))
                        .await;
                    let _ = db.finish_match(job.match_id, None).await;
                    continue;
                }
            };

            // Load bot code for each participant
            let mut players = Vec::new();
            let mut version_ids = Vec::new();
            let mut load_error = None;

            for p in &participants {
                match db.get_bot_version_by_id(p.bot_version_id).await {
                    Ok(Some(v)) => {
                        let name = p
                            .bot_name
                            .clone()
                            .unwrap_or_else(|| format!("Bot v{}", v.version));
                        players.push(PlayerEntry { name, code: v.code });
                        version_ids.push(p.bot_version_id);
                    }
                    Ok(None) => {
                        load_error = Some(format!("Bot version {} not found", p.bot_version_id));
                        break;
                    }
                    Err(e) => {
                        load_error = Some(format!(
                            "DB error loading version {}: {e}",
                            p.bot_version_id
                        ));
                        break;
                    }
                }
            }

            if let Some(err) = load_error {
                tracing::error!("Queue worker: {err}");
                let _ = db.fail_queue_job(job.id, &err).await;
                let _ = db.finish_match(job.match_id, None).await;
                continue;
            }

            // Resolve map (deserialize map_params if present)
            let map_params: Option<crate::api::MapParamsRequest> = job
                .map_params
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok());
            let world = match resolve_map(&maps_dir, &job.map, map_params.as_ref()) {
                Ok(w) => w,
                Err(e) => {
                    tracing::error!("Queue worker: invalid map for match {}: {e}", job.match_id);
                    let _ = db
                        .fail_queue_job(job.id, &format!("Invalid map: {e}"))
                        .await;
                    let _ = db.finish_match(job.match_id, None).await;
                    continue;
                }
            };

            let format = if players.len() == 2 {
                "1v1".to_string()
            } else {
                "ffa".to_string()
            };

            let match_id = job.match_id;
            let job_id = job.id;

            // Build completion callback
            let db_complete = db.clone();
            let version_ids_cb = version_ids.clone();
            let format_cb = format.clone();
            let on_complete = move |result: crate::engine::server::GameResult| {
                let db = db_complete;
                let version_ids = version_ids_cb;
                let format = format_cb;
                let rt = tokio::runtime::Handle::current();
                rt.spawn(async move {
                    // Run all post-game bookkeeping
                    run_game_completion(&db, match_id, &version_ids, &format, &result).await;

                    // Mark queue job complete
                    if let Err(e) = db.complete_queue_job(job_id).await {
                        tracing::error!("Failed to complete queue job {job_id}: {e}");
                    }

                    // Update queue depth metric
                    if let Ok(status) = db.queue_status().await {
                        metrics::GAME_QUEUE_DEPTH.set(status.pending);
                    }
                });
            };

            let spawned = worker_pool.spawn_game(
                world,
                players,
                6000,
                Some(match_id),
                version_ids,
                on_complete,
            );

            if !spawned {
                tracing::warn!("Worker pool rejected game for match {match_id}");
                let _ = db.fail_queue_job(job_id, "Worker pool at capacity").await;
            } else {
                // Update queue depth metric
                if let Ok(status) = db.queue_status().await {
                    metrics::GAME_QUEUE_DEPTH.set(status.pending);
                }
            }
        }
    });
}

/// Shared game completion logic used by both the queue worker (headless)
/// and live game callbacks.
pub async fn run_game_completion(
    db: &Database,
    match_id: i64,
    version_ids: &[i64],
    format: &str,
    result: &crate::engine::server::GameResult,
) {
    // 1. Save replay
    if let Err(e) = db
        .save_replay(match_id, &result.replay_data, result.tick_count)
        .await
    {
        tracing::error!("Failed to save replay for match {match_id}: {e}");
    }

    // 1b. Mark faulty bot versions
    for &vid in &result.failed_bot_version_ids {
        if let Err(e) = db.mark_version_faulty(vid, true).await {
            tracing::error!("Failed to mark version {vid} as faulty: {e}");
        }
    }

    // 2. Determine winner bot_version_id
    let winner_version_id = result
        .winner_player_index
        .and_then(|idx| version_ids.get(idx).copied());

    // 3. Finish match
    if let Err(e) = db.finish_match(match_id, winner_version_id).await {
        tracing::error!("Failed to finish match {match_id}: {e}");
    }

    // 3b. Create notifications
    if let Ok(owner_ids) = db.get_match_participant_owner_ids(match_id).await {
        let winner_name = if let Some(wid) = winner_version_id {
            if let Ok(Some(v)) = db.get_bot_version_by_id(wid).await {
                if let Ok(Some(b)) = db.get_bot(v.bot_id).await {
                    Some(format!("{} v{}", b.name, v.version))
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        let title = "Match completed".to_string();
        let message = match winner_name {
            Some(name) => format!("Match #{match_id} finished. Winner: {name}"),
            None => format!("Match #{match_id} finished (draw)"),
        };
        let data = serde_json::json!({ "match_id": match_id }).to_string();
        for owner_id in owner_ids {
            let _ = db
                .create_notification(owner_id, "match_complete", &title, &message, Some(&data))
                .await;
        }
    }

    // 4. Get participants and update stats + Elo
    let participants = match db.get_match_participants(match_id).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!("Failed to get participants for match {match_id}: {e}");
            return;
        }
    };

    // Update per-participant stats
    for p in participants.iter() {
        let ps = result
            .player_scores
            .iter()
            .find(|s| s.bot_version_id == p.bot_version_id);
        let score = ps.map(|s| s.score).unwrap_or(0);
        let spawned = ps.map(|s| s.creatures_spawned).unwrap_or(0);
        let killed = ps.map(|s| s.creatures_killed).unwrap_or(0);
        let lost_c = ps.map(|s| s.creatures_lost).unwrap_or(0);
        let won = winner_version_id == Some(p.bot_version_id);
        let lost = winner_version_id.is_some() && !won;
        let draw = winner_version_id.is_none();
        let placement = if won {
            1
        } else if lost {
            2
        } else {
            0
        };

        let _ = db
            .update_match_participant(
                p.id,
                score,
                Some(placement),
                Some(0),
                Some(0),
                spawned,
                killed,
                lost_c,
            )
            .await;

        let _ = db
            .update_version_stats(
                p.bot_version_id,
                won,
                lost,
                draw,
                score,
                spawned,
                killed,
                lost_c,
            )
            .await;
    }

    // Elo calculation for 1v1
    if format == "1v1" && participants.len() == 2 {
        let p0 = &participants[0];
        let p1 = &participants[1];
        let (v0, v1) = match (
            db.get_bot_version_by_id(p0.bot_version_id).await,
            db.get_bot_version_by_id(p1.bot_version_id).await,
        ) {
            (Ok(Some(v0)), Ok(Some(v1))) => (v0, v1),
            _ => return,
        };

        let outcome_0 = if winner_version_id == Some(p0.bot_version_id) {
            crate::elo::Outcome::Win
        } else if winner_version_id == Some(p1.bot_version_id) {
            crate::elo::Outcome::Loss
        } else {
            crate::elo::Outcome::Draw
        };
        let outcome_1 = match outcome_0 {
            crate::elo::Outcome::Win => crate::elo::Outcome::Loss,
            crate::elo::Outcome::Loss => crate::elo::Outcome::Win,
            crate::elo::Outcome::Draw => crate::elo::Outcome::Draw,
        };

        let new_elo_0 =
            crate::elo::calculate_new_rating(v0.elo_1v1, v1.elo_1v1, outcome_0, v0.games_played);
        let new_elo_1 =
            crate::elo::calculate_new_rating(v1.elo_1v1, v0.elo_1v1, outcome_1, v1.games_played);

        let ps0 = result
            .player_scores
            .iter()
            .find(|s| s.bot_version_id == p0.bot_version_id);
        let ps1 = result
            .player_scores
            .iter()
            .find(|s| s.bot_version_id == p1.bot_version_id);

        let _ = db
            .update_match_participant(
                p0.id,
                ps0.map(|s| s.score).unwrap_or(0),
                Some(if outcome_0 == crate::elo::Outcome::Win {
                    1
                } else {
                    2
                }),
                Some(v0.elo_1v1),
                Some(new_elo_0),
                ps0.map(|s| s.creatures_spawned).unwrap_or(0),
                ps0.map(|s| s.creatures_killed).unwrap_or(0),
                ps0.map(|s| s.creatures_lost).unwrap_or(0),
            )
            .await;
        let _ = db
            .update_match_participant(
                p1.id,
                ps1.map(|s| s.score).unwrap_or(0),
                Some(if outcome_1 == crate::elo::Outcome::Win {
                    1
                } else {
                    2
                }),
                Some(v1.elo_1v1),
                Some(new_elo_1),
                ps1.map(|s| s.creatures_spawned).unwrap_or(0),
                ps1.map(|s| s.creatures_killed).unwrap_or(0),
                ps1.map(|s| s.creatures_lost).unwrap_or(0),
            )
            .await;

        let _ = db.update_version_elo(p0.bot_version_id, new_elo_0).await;
        let _ = db.update_version_elo(p1.bot_version_id, new_elo_1).await;
    }

    // FFA placement scoring
    if format == "ffa" && participants.len() > 2 {
        let mut sorted: Vec<&crate::db::MatchParticipant> = participants.iter().collect();
        sorted.sort_by(|a, b| {
            let score_a = result
                .player_scores
                .iter()
                .find(|s| s.bot_version_id == a.bot_version_id)
                .map(|s| s.score)
                .unwrap_or(0);
            let score_b = result
                .player_scores
                .iter()
                .find(|s| s.bot_version_id == b.bot_version_id)
                .map(|s| s.score)
                .unwrap_or(0);
            score_b.cmp(&score_a)
        });

        let n_players = participants.len() as i32;
        for (placement_idx, p) in sorted.iter().enumerate() {
            let placement = (placement_idx + 1) as i32;
            let points = crate::elo::ffa_placement_points(placement, n_players);
            let _ = db.update_version_ffa_stats(p.bot_version_id, points).await;
        }
    }

    // Tournament advancement
    if let Ok(Some((tournament_id, round))) = db.get_tournament_for_match(match_id).await {
        // Save tournament results for each participant
        for p in participants.iter() {
            let ps = result
                .player_scores
                .iter()
                .find(|s| s.bot_version_id == p.bot_version_id);
            let score = ps.map(|s| s.score).unwrap_or(0);
            let _ = db
                .add_tournament_result(
                    tournament_id,
                    p.player_slot,
                    p.bot_version_id,
                    score,
                    ps.map(|s| s.creatures_spawned).unwrap_or(0),
                    ps.map(|s| s.creatures_killed).unwrap_or(0),
                    ps.map(|s| s.creatures_lost).unwrap_or(0),
                )
                .await;
        }

        // Check if all matches in this round are finished
        if let Ok(round_matches) = db
            .list_tournament_matches_by_round(tournament_id, round)
            .await
        {
            let mut all_finished = true;
            for tm in &round_matches {
                if let Ok(Some(m)) = db.get_match(tm.match_id).await {
                    if m.status != "finished" {
                        all_finished = false;
                        break;
                    }
                }
            }

            if all_finished {
                if let Ok(Some(tournament)) = db.get_tournament(tournament_id).await {
                    let fmt =
                        crate::tournament::TournamentFormat::from_str_name(&tournament.format)
                            .unwrap_or(crate::tournament::TournamentFormat::RoundRobin);
                    let n_participants = db
                        .list_tournament_entries(tournament_id)
                        .await
                        .map(|e| e.len())
                        .unwrap_or(0);
                    let num_rounds = crate::tournament::total_rounds(&fmt, n_participants);

                    if round < num_rounds as i32 {
                        advance_tournament_round(
                            db,
                            tournament_id,
                            round + 1,
                            &fmt,
                            &tournament.map,
                        )
                        .await;
                    } else {
                        let _ = db.update_tournament_status(tournament_id, "finished").await;
                    }
                }
            }
        }
    }
}

/// Advance a tournament to the next round by creating and queuing new matches.
async fn advance_tournament_round(
    db: &Database,
    tournament_id: i64,
    next_round: i32,
    format: &crate::tournament::TournamentFormat,
    map: &str,
) {
    use crate::tournament::*;

    let entries = match db.list_tournament_entries(tournament_id).await {
        Ok(e) => e,
        Err(_) => return,
    };
    let all_version_ids: Vec<i64> = entries.iter().map(|e| e.bot_version_id).collect();

    let pairings = match format {
        TournamentFormat::SingleElimination => {
            let prev_matches = db
                .list_tournament_matches_by_round(tournament_id, next_round - 1)
                .await
                .unwrap_or_default();
            let mut winners = Vec::new();
            for tm in &prev_matches {
                if let Ok(Some(m)) = db.get_match(tm.match_id).await {
                    if let Some(winner_id) = m.winner_bot_version_id {
                        winners.push(winner_id);
                    }
                }
            }
            generate_single_elimination_bracket(&winners)
        }
        TournamentFormat::Swiss { .. } => {
            let standings = db
                .get_tournament_standings(tournament_id)
                .await
                .unwrap_or_default();
            let standings_tuples: Vec<(i64, f64)> = standings
                .iter()
                .map(|s| (s.bot_version_id, s.total_score as f64))
                .collect();
            generate_swiss_pairings(&all_version_ids, &standings_tuples, next_round as usize)
        }
        TournamentFormat::RoundRobin => return,
    };

    if pairings.is_empty() {
        let _ = db.update_tournament_status(tournament_id, "finished").await;
        return;
    }

    for (vid_a, vid_b) in &pairings {
        let m = match db.create_match("1v1", map).await {
            Ok(m) => m,
            Err(_) => continue,
        };
        let _ = db.add_match_participant(m.id, *vid_a, 0).await;
        let _ = db.add_match_participant(m.id, *vid_b, 1).await;
        let _ = db
            .add_tournament_match(tournament_id, m.id, next_round)
            .await;

        // Enqueue via DB — tournament matches get priority 10
        if let Err(e) = db.enqueue_game(m.id, Some(map), 10, None).await {
            tracing::error!("Failed to enqueue tournament match {}: {e}", m.id);
        }
    }

    let _ = db.update_tournament_round(tournament_id, next_round).await;
}
