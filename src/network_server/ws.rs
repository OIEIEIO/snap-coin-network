// =============================================================================
// File: src/network_server/ws.rs
// Project: snap-coin-network / src/network_server/
// Version: 0.1.1
// Description: WebSocket handler — broadcasts NetworkEvents to connected frontend
//              clients. Added 30s keepalive ping to prevent nginx proxy timeout
//              from dropping idle connections.
// Modified: 2026-03-28
// =============================================================================

use axum::{
    extract::{State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};
use crate::telemetry::events::NetworkEvent;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(tx): State<broadcast::Sender<NetworkEvent>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, tx))
}

async fn handle_socket(socket: WebSocket, tx: broadcast::Sender<NetworkEvent>) {
    let mut rx           = tx.subscribe();
    let (mut sender, _)  = socket.split();
    let mut ping_ticker  = interval(Duration::from_secs(30));
    ping_ticker.tick().await; // skip immediate first tick

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(event) => {
                        if let Ok(json) = serde_json::to_string(&event) {
                            if sender.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            _ = ping_ticker.tick() => {
                if sender.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
        }
    }
}

// =============================================================================
// File: src/network_server/ws.rs
// Project: snap-coin-network / src/network_server/
// Created: 2026-03-28T00:00:00Z
// =============================================================================