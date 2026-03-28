// File: src/telemetry/state.rs
// Project: snap-coin-network / src/telemetry/
// Version: 0.1.2
// Description: In-memory NetworkState — active peers, history, counters, self geo

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::events::now_unix;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerRecord {
    pub addr:           SocketAddr,
    pub inbound:        bool,
    pub connected_at:   u64,
    pub disconnected_at: Option<u64>,
    pub lat:            Option<f64>,
    pub lon:            Option<f64>,
    pub country:        Option<String>,
    pub city:           Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub total_connections:    u64,
    pub total_disconnections: u64,
    pub blocks_seen:          u64,
    pub transactions_seen:    u64,
    pub uptime_since:         u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfGeo {
    pub lat:     f64,
    pub lon:     f64,
    pub country: Option<String>,
    pub city:    Option<String>,
}

#[derive(Debug)]
pub struct NetworkState {
    pub active_peers: RwLock<HashMap<SocketAddr, PeerRecord>>,
    pub peer_history: RwLock<Vec<PeerRecord>>,
    pub stats:        RwLock<NetworkStats>,
    pub self_geo:     RwLock<Option<SelfGeo>>,
}

pub type SharedNetworkState = Arc<NetworkState>;

impl NetworkState {
    pub fn new() -> SharedNetworkState {
        Arc::new(NetworkState {
            active_peers: RwLock::new(HashMap::new()),
            peer_history: RwLock::new(Vec::new()),
            stats: RwLock::new(NetworkStats {
                total_connections:    0,
                total_disconnections: 0,
                blocks_seen:          0,
                transactions_seen:    0,
                uptime_since:         now_unix(),
            }),
            self_geo: RwLock::new(None),
        })
    }

    pub async fn set_self_geo(&self, geo: SelfGeo) {
        *self.self_geo.write().await = Some(geo);
    }

    pub async fn peer_connected(
        &self,
        addr:    SocketAddr,
        inbound: bool,
        lat:     Option<f64>,
        lon:     Option<f64>,
        country: Option<String>,
        city:    Option<String>,
    ) {
        let record = PeerRecord {
            addr,
            inbound,
            connected_at:    now_unix(),
            disconnected_at: None,
            lat,
            lon,
            country,
            city,
        };
        self.active_peers.write().await.insert(addr, record);
        self.stats.write().await.total_connections += 1;
    }

    pub async fn peer_disconnected(&self, addr: SocketAddr) {
        let mut active = self.active_peers.write().await;
        if let Some(mut record) = active.remove(&addr) {
            record.disconnected_at = Some(now_unix());
            self.peer_history.write().await.push(record);
        }
        self.stats.write().await.total_disconnections += 1;
    }

    pub async fn block_seen(&self) {
        self.stats.write().await.blocks_seen += 1;
    }

    pub async fn transaction_seen(&self) {
        self.stats.write().await.transactions_seen += 1;
    }
}

// File: src/telemetry/state.rs / snap-coin-network / 2026-03-27