// File: src/telemetry/events.rs
// Project: snap-coin-network / src/telemetry/
// Version: 0.1.0
// Description: NetworkEvent enum — all observable peer and chain events

use std::net::SocketAddr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NetworkEvent {
    PeerConnected {
        addr: SocketAddr,
        inbound: bool,
        timestamp: u64,
    },
    PeerDisconnected {
        addr: SocketAddr,
        inbound: bool,
        timestamp: u64,
    },
    BlockSeen {
        timestamp: u64,
    },
    TransactionSeen {
        timestamp: u64,
    },
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// File: src/telemetry/events.rs / snap-coin-network / 2026-03-27