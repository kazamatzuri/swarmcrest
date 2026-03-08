// WebSocket handler for game state streaming.
// TODO: Phase 10.3 - Add live match chat support. This requires moderation
// infrastructure (rate limiting, filtering, user muting) before implementation.
// When added, client messages should be parsed as chat messages and broadcast
// to other spectators via a separate chat channel.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, Query, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::auth;
use crate::metrics;

use super::AppState;

const MAX_WS_CONNECTIONS_PER_IP: usize = 10;

static WS_CONNECTIONS: std::sync::LazyLock<Arc<Mutex<HashMap<std::net::IpAddr, usize>>>> =
    std::sync::LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(serde::Deserialize)]
pub struct WsQuery {
    token: Option<String>,
}

/// WebSocket upgrade handler for game state streaming.
pub async fn ws_game(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    // In production mode, require a valid token
    if !crate::config::is_local_mode() {
        let valid = match &query.token {
            Some(token) => auth::verify_token(token).is_ok(),
            None => false,
        };
        if !valid {
            return (
                StatusCode::UNAUTHORIZED,
                "Valid token required for WebSocket connection",
            )
                .into_response();
        }
    }

    // Per-IP connection limit
    let ip = addr.ip();
    {
        let conns = WS_CONNECTIONS.lock().unwrap();
        let count = conns.get(&ip).copied().unwrap_or(0);
        if count >= MAX_WS_CONNECTIONS_PER_IP {
            return (
                StatusCode::TOO_MANY_REQUESTS,
                "Too many WebSocket connections from this IP",
            )
                .into_response();
        }
    }

    ws.on_upgrade(move |socket| handle_ws(socket, state, ip))
        .into_response()
}

async fn handle_ws(mut socket: WebSocket, state: AppState, ip: std::net::IpAddr) {
    // Track connection
    {
        let mut conns = WS_CONNECTIONS.lock().unwrap();
        *conns.entry(ip).or_insert(0) += 1;
    }

    metrics::CONNECTED_WEBSOCKETS.inc();
    let mut rx = state.game_server.subscribe();

    // Send cached world snapshot so late joiners see the map immediately.
    if let Some(world_json) = state.game_server.world_json() {
        if socket.send(Message::Text(world_json.into())).await.is_err() {
            metrics::CONNECTED_WEBSOCKETS.dec();
            return;
        }
        metrics::WEBSOCKET_MESSAGES_SENT_TOTAL.inc();
    }

    // Forward all broadcast messages to the WebSocket client.
    // When the client disconnects or the broadcast channel closes, we stop.
    loop {
        tokio::select! {
            // Game message from broadcast channel
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            // Client disconnected
                            break;
                        }
                        metrics::WEBSOCKET_MESSAGES_SENT_TOTAL.inc();
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // Channel closed, game ended
                        break;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WebSocket client lagged, skipped {n} messages");
                        // Continue receiving
                    }
                }
            }
            // Client message (we mostly ignore, but detect disconnect)
            result = socket.recv() => {
                match result {
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    _ => {
                        // Ignore other client messages for now
                    }
                }
            }
        }
    }

    // Decrement per-IP connection count
    {
        let mut conns = WS_CONNECTIONS.lock().unwrap();
        if let Some(count) = conns.get_mut(&ip) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                conns.remove(&ip);
            }
        }
    }
    metrics::CONNECTED_WEBSOCKETS.dec();
}
