// File: src/telemetry/chain_watcher.rs
// Project: snap-coin-network / src/telemetry/
// Version: 0.1.0
// Description: Subscribes to chain_events broadcast — emits BlockSeen and TransactionSeen

use std::sync::Arc;

use log::info;
use tokio::sync::broadcast;

use snap_coin::{
    full_node::node_state::SharedNodeState,
    node::chain_events::ChainEvent,
};

use super::{
    events::{NetworkEvent, now_unix},
    journal::Journal,
    state::SharedNetworkState,
};

pub async fn run_chain_watcher(
    node_state: SharedNodeState,
    net_state: SharedNetworkState,
    journal: Arc<Journal>,
    event_tx: broadcast::Sender<NetworkEvent>,
) {
    let mut rx = node_state.chain_events.subscribe();

    loop {
        match rx.recv().await {
            Ok(chain_event) => {
                let net_event = match chain_event {
                    ChainEvent::Block { .. } => {
                        info!("Chain event: block seen");
                        net_state.block_seen().await;
                        NetworkEvent::BlockSeen {
                            timestamp: now_unix(),
                        }
                    }
                    ChainEvent::Transaction { .. } => {
                        info!("Chain event: transaction seen");
                        net_state.transaction_seen().await;
                        NetworkEvent::TransactionSeen {
                            timestamp: now_unix(),
                        }
                    }
                    ChainEvent::TransactionExpiration { .. } => {
                        continue;
                    }
                };

                let _ = journal.write(&net_event).await;
                let _ = event_tx.send(net_event);
            }
            Err(broadcast::error::RecvError::Lagged(n)) => {
                info!("Chain watcher lagged by {} events", n);
            }
            Err(broadcast::error::RecvError::Closed) => {
                info!("Chain events channel closed — watcher exiting");
                break;
            }
        }
    }
}

// File: src/telemetry/chain_watcher.rs / snap-coin-network / 2026-03-27