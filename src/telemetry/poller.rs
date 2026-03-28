// File: src/telemetry/poller.rs
// Project: snap-coin-network / src/telemetry/
// Version: 0.1.0
// Description: Diff-poll loop on connected_peers — emits connect/disconnect events

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};

use log::info;
use tokio::sync::broadcast;

use snap_coin::full_node::node_state::SharedNodeState;

use super::{
    events::{NetworkEvent, now_unix},
    geo::GeoDb,
    state::SharedNetworkState,
    journal::Journal,
};

pub async fn run_poller(
    node_state: SharedNodeState,
    net_state: SharedNetworkState,
    journal: Arc<Journal>,
    geo: Arc<GeoDb>,
    event_tx: broadcast::Sender<NetworkEvent>,
) {
    let mut known: HashMap<SocketAddr, bool> = HashMap::new();
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        interval.tick().await;

        let current: HashMap<SocketAddr, bool> = node_state
            .connected_peers
            .read()
            .await
            .iter()
            .map(|(addr, handle)| (*addr, handle.is_client))
            .collect();

        // Detect new connections
        for (addr, is_client) in &current {
            if !known.contains_key(addr) {
                let inbound = !is_client;
                let (lat, lon, country, city) = geo.lookup(addr.ip());

                let event = NetworkEvent::PeerConnected {
                    addr: *addr,
                    inbound,
                    timestamp: now_unix(),
                };

                info!("Peer connected: {} inbound={}", addr, inbound);
                net_state
                    .peer_connected(*addr, inbound, lat, lon, country, city)
                    .await;
                let _ = journal.write(&event).await;
                let _ = event_tx.send(event);
            }
        }

        // Detect disconnections
        for (addr, is_client) in &known {
            if !current.contains_key(addr) {
                let inbound = !is_client;

                let event = NetworkEvent::PeerDisconnected {
                    addr: *addr,
                    inbound,
                    timestamp: now_unix(),
                };

                info!("Peer disconnected: {}", addr);
                net_state.peer_disconnected(*addr).await;
                let _ = journal.write(&event).await;
                let _ = event_tx.send(event);
            }
        }

        known = current;
    }
}

// File: src/telemetry/poller.rs / snap-coin-network / 2026-03-27