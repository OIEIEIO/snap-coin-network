// File: src/network_server/mod.rs
// Project: snap-coin-network / src/network_server/
// Version: 0.1.2
// Description: Axum router — wires REST, WebSocket, and static file serving

use axum::{Router, routing::get};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

use crate::telemetry::{events::NetworkEvent, state::SharedNetworkState};

pub mod geo;
pub mod rest;
pub mod ws;

pub async fn start_network_server(
    port: u16,
    net_state: SharedNetworkState,
    event_tx: broadcast::Sender<NetworkEvent>,
) -> anyhow::Result<()> {
    let rest_state = net_state.clone();
    let ws_state   = event_tx.clone();

    let app = Router::new()
        .route("/api/summary", get(rest::summary).with_state(rest_state.clone()))
        .route("/api/peers",   get(rest::peers).with_state(rest_state.clone()))
        .route("/api/history", get(rest::history).with_state(rest_state))
        .route("/ws",          get(ws::ws_handler).with_state(ws_state))
        .fallback_service(ServeDir::new("static"));

    let addr     = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    log::info!("Network server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

// File: src/network_server/mod.rs / snap-coin-network / 2026-03-27