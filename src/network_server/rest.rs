// File: src/network_server/rest.rs
// Project: snap-coin-network / src/network_server/
// Version: 0.1.2
// Description: REST endpoints — /api/summary, /api/peers, /api/history

use axum::{Json, extract::State};
use serde_json::{Value, json};

use crate::telemetry::state::SharedNetworkState;

pub async fn summary(State(net_state): State<SharedNetworkState>) -> Json<Value> {
    let stats      = net_state.stats.read().await.clone();
    let active     = net_state.active_peers.read().await.len();
    let self_geo   = net_state.self_geo.read().await.clone();

    let self_json = match self_geo {
        Some(g) => json!({
            "lat":     g.lat,
            "lon":     g.lon,
            "country": g.country,
            "city":    g.city,
        }),
        None => json!(null),
    };

    Json(json!({
        "active_peers":        active,
        "total_connections":   stats.total_connections,
        "total_disconnections": stats.total_disconnections,
        "blocks_seen":         stats.blocks_seen,
        "transactions_seen":   stats.transactions_seen,
        "uptime_since":        stats.uptime_since,
        "self":                self_json,
    }))
}

pub async fn peers(State(net_state): State<SharedNetworkState>) -> Json<Value> {
    let active = net_state.active_peers.read().await;
    let peers: Vec<Value> = active.values().map(|p| json!({
        "addr":         p.addr.to_string(),
        "inbound":      p.inbound,
        "connected_at": p.connected_at,
        "lat":          p.lat,
        "lon":          p.lon,
        "country":      p.country,
        "city":         p.city,
    })).collect();
    Json(json!({ "peers": peers }))
}

pub async fn history(State(net_state): State<SharedNetworkState>) -> Json<Value> {
    let hist = net_state.peer_history.read().await;
    let records: Vec<Value> = hist.iter().rev().take(200).map(|p| json!({
        "addr":             p.addr.to_string(),
        "inbound":          p.inbound,
        "connected_at":     p.connected_at,
        "disconnected_at":  p.disconnected_at,
        "country":          p.country,
        "city":             p.city,
    })).collect();
    Json(json!({ "history": records }))
}

// File: src/network_server/rest.rs / snap-coin-network / 2026-03-27