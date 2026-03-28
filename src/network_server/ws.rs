// File: src/network_server/ws.rs
// Project: snap-coin-network / src/network_server/
// Version: 0.1.0
// Description: WebSocket handler — broadcasts NetworkEvents to connected frontend clients

use axum::{
    extract::{State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use tokio::sync::broadcast;

use crate::telemetry::events::NetworkEvent;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(tx): State<broadcast::Sender<NetworkEvent>>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, tx))
}

async fn handle_socket(socket: WebSocket, tx: broadcast::Sender<NetworkEvent>) {
    let mut rx = tx.subscribe();
    let (mut sender, _receiver) = socket.split();

    while let Ok(event) = rx.recv().await {
        if let Ok(json) = serde_json::to_string(&event) {
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    }
}

// File: src/network_server/ws.rs / snap-coin-network / 2026-03-27