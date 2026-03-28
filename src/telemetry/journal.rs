// File: src/telemetry/journal.rs
// Project: snap-coin-network / src/telemetry/
// Version: 0.1.1
// Description: Append-only jsonl event journal writer

use std::path::PathBuf;

use anyhow::Result;
use tokio::{
    fs::OpenOptions,
    io::AsyncWriteExt,
    sync::Mutex,
};

use super::events::NetworkEvent;

pub struct Journal {
    file: Mutex<tokio::fs::File>,
}

impl Journal {
    pub async fn open(node_path: &str) -> Result<Self> {
        let path = PathBuf::from(node_path).join("network-events.jsonl");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await?;
        log::info!("Network event journal: {}", path.display());
        Ok(Journal {
            file: Mutex::new(file),
        })
    }

    pub async fn write(&self, event: &NetworkEvent) -> Result<()> {
        let mut line = serde_json::to_string(event)?;
        line.push('\n');
        self.file.lock().await.write_all(line.as_bytes()).await?;
        Ok(())
    }
}

// File: src/telemetry/journal.rs / snap-coin-network / 2026-03-27