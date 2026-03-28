// File: src/main.rs
// Project: snap-coin-network
// Version: 0.1.2
// Description: Snap Coin Network node entrypoint — full node + telemetry + network server

use std::{net::IpAddr, sync::Arc, time::Duration};

use anyhow::anyhow;
use clap::Parser;
use log::info;
use tokio::{net::lookup_host, sync::broadcast, time::sleep};

use snap_coin::{
    build_block,
    crypto::{Hash, randomx_optimized_mode},
    economics::DEV_WALLET,
    full_node::{
        accept_block, api_server::FullNodeApiServer, auto_peer::start_auto_peer,
        auto_reconnect::start_auto_reconnect, connect_peer, create_full_node, ibd::ibd_blockchain,
        p2p_server::start_p2p_server,
    },
};

use crate::tui::run_tui;

mod deprecated_block_store;
mod network_server;
mod telemetry;
mod tui;
mod upgrade;

#[derive(Parser, Debug)]
#[command(name = "snap-coin-network", version)]
struct Args {
    #[arg(long, value_delimiter = ',', short = 'P')]
    peers: Vec<String>,

    #[arg(long, value_delimiter = ',', short = 'r')]
    reserved_ips: Vec<String>,

    #[arg(long, short = 'A')]
    advertise: Option<String>,

    #[arg(long, default_value = "./node-mainnet", short = 'd')]
    node_path: String,

    #[arg(long)]
    no_api: bool,

    #[arg(long, default_value_t = 3003, short = 'a')]
    api_port: u16,

    #[arg(long, default_value_t = 8998, short = 'p')]
    node_port: u16,

    #[arg(long, default_value_t = 3030)]
    network_port: u16,

    #[arg(long, default_value = "./data/GeoLite2-City.mmdb")]
    geo_db: String,

    #[arg(long)]
    create_genesis: bool,

    #[arg(long, short = 'H')]
    headless: bool,

    #[arg(long)]
    no_ibd: bool,

    #[arg(long)]
    full_ibd: bool,

    #[arg(long)]
    no_auto_peer: bool,

    #[arg(long, short = 'T')]
    ibd_turbo: bool,

    #[arg(long, default_value_t = 0, short = 't')]
    ibd_threads: usize,

    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    if args.debug {
        use tracing_subscriber::prelude::*;
        if tracing_subscriber::registry()
            .with(console_subscriber::spawn())
            .try_init()
            .is_err()
        {}
    }

    if !args.full_ibd {
        println!("IBD in normal mode. Will not validate transaction hashes that are > 500 blocks away from head.");
    }

    let ibd_threads = if args.ibd_threads != 0 {
        args.ibd_threads
    } else {
        std::thread::available_parallelism()?.get()
    };

    if !args.full_ibd {
        println!("IBD will hash on {ibd_threads} threads.");
    }

    if args.ibd_turbo {
        randomx_optimized_mode(true);
        Hash::new(b"INIT");
    } else {
        println!("RandomX started in light mode.");
    }

    upgrade::upgrade(&args.node_path).await?;

    let mut resolved_peers = Vec::new();
    for seed in &args.peers {
        match lookup_host(seed).await {
            Ok(addrs) => {
                if let Some(addr) = addrs.into_iter().next() {
                    resolved_peers.push(addr);
                }
            }
            Err(_) => return Err(anyhow!("Failed to resolve or parse seed peer: {seed}")),
        }
    }

    let mut parsed_reserved_ips: Vec<IpAddr> = vec![];
    for reserved_ip in args.reserved_ips {
        parsed_reserved_ips.push(reserved_ip.parse().expect("Reserved ip is invalid"));
    }

    let advertised_ip = if let Some(addr_str) = args.advertise {
        Some(lookup_host(addr_str).await?.next().unwrap())
    } else {
        None
    };

    // --- Telemetry setup ---
    let geo       = Arc::new(telemetry::geo::GeoDb::open(&args.geo_db));
    let net_state = telemetry::state::NetworkState::new();
    let journal   = Arc::new(telemetry::journal::Journal::open(&args.node_path).await?);
    let (event_tx, _) = broadcast::channel::<telemetry::events::NetworkEvent>(256);

    // --- Resolve self geo ---
    {
        use std::net::UdpSocket;
        let self_ip: Option<IpAddr> = if let Some(addr) = advertised_ip {
            Some(addr.ip())
        } else {
            UdpSocket::bind("0.0.0.0:0")
                .ok()
                .and_then(|s| s.connect("8.8.8.8:80").ok().map(|_| s))
                .and_then(|s| s.local_addr().ok())
                .map(|a| a.ip())
        };

        if let Some(ip) = self_ip {
            let (lat, lon, country, city) = geo.lookup(ip);
            if let (Some(lat), Some(lon)) = (lat, lon) {
                net_state.set_self_geo(telemetry::state::SelfGeo {
                    lat,
                    lon,
                    country,
                    city,
                }).await;
                info!("Self geo resolved: {},{}", lat, lon);
            } else {
                info!("Self geo: IP {} not found in geo db", ip);
            }
        }
    }

    // --- Full node ---
    let (blockchain, node_state, latest_log_file) =
        create_full_node(&args.node_path, !args.headless, advertised_ip);

    for initial_peer in &resolved_peers {
        connect_peer(*initial_peer, &blockchain, &node_state).await?;
    }

    *node_state.is_syncing.write().await = true;

    if !args.no_api {
        sleep(Duration::from_secs(1)).await;
        let api_server =
            FullNodeApiServer::new(args.api_port as u32, blockchain.clone(), node_state.clone());
        api_server.listen().await?;
    }

    if args.create_genesis {
        let mut genesis = build_block(&*blockchain, &vec![], DEV_WALLET).await?;
        #[allow(deprecated)]
        genesis.compute_pow()?;
        accept_block(&blockchain, &node_state, genesis).await?;
    }

    if !resolved_peers.is_empty() && !args.no_ibd {
        let blockchain = blockchain.clone();
        let node_state = node_state.clone();
        tokio::spawn(async move {
            sleep(Duration::from_secs(1)).await;
            info!(
                "Blockchain sync status {:?}",
                ibd_blockchain(node_state.clone(), blockchain, args.full_ibd, ibd_threads).await
            );
            *node_state.is_syncing.write().await = false;
        });
    } else {
        *node_state.is_syncing.write().await = false;
    }

    if !resolved_peers.is_empty() {
        let _ = start_auto_reconnect(
            node_state.clone(),
            blockchain.clone(),
            resolved_peers.clone(),
            args.full_ibd,
            ibd_threads,
        );
    }

    if !args.no_auto_peer {
        let _ = start_auto_peer(node_state.clone(), blockchain.clone(), parsed_reserved_ips);
    }

    // --- Start telemetry tasks ---
    {
        let ns  = node_state.clone();
        let nst = net_state.clone();
        let j   = journal.clone();
        let g   = geo.clone();
        let tx  = event_tx.clone();
        tokio::spawn(async move {
            telemetry::poller::run_poller(ns, nst, j, g, tx).await;
        });
    }

    {
        let ns  = node_state.clone();
        let nst = net_state.clone();
        let j   = journal.clone();
        let tx  = event_tx.clone();
        tokio::spawn(async move {
            telemetry::chain_watcher::run_chain_watcher(ns, nst, j, tx).await;
        });
    }

    // --- Start network server ---
    {
        let nst  = net_state.clone();
        let tx   = event_tx.clone();
        let port = args.network_port;
        tokio::spawn(async move {
            if let Err(e) = network_server::start_network_server(port, nst, tx).await {
                log::error!("Network server error: {e}");
            }
        });
    }

    let p2p_server_handle =
        start_p2p_server(args.node_port, blockchain.clone(), node_state.clone()).await?;

    if args.headless {
        info!("{:?}", p2p_server_handle.await);
    } else {
        run_tui(node_state, blockchain, args.node_port, latest_log_file).await?;
    }

    Ok(())
}

// File: src/main.rs / snap-coin-network / 2026-03-27