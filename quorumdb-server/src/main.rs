use std::sync::Arc;
use quorumdb_core::KvStore;

mod server;
mod protocol;
mod handler;
mod cluster_config;
mod cluster_node;
mod raft_handler;
mod raft_tick_loop;
mod peer_server;

use cluster_config::ClusterConfig;
use cluster_node::ClusterNode;

#[tokio::main]
async fn main() {
    // Load cluster configuration
    let config = ClusterConfig::from_env();

    // Initialize state machine (KvStore)
    let store = Arc::new(KvStore::new());

    if let Err(e) = store.init_wal(&format!("{}.wal", config.node_id)).await {
        eprintln!("Failed to initialize WAL: {}", e);
        std::process::exit(1);
    }

    // Create cluster node
    let node = Arc::new(ClusterNode::new(config.clone(), store.clone()).await);

    println!("=== QuorumDB Cluster Node ===");
    println!("Node ID: {}", node.config.node_id);
    println!("Client Address: {}", node.config.client_addr());
    println!("Raft Address: {}", node.config.raft_addr());
    println!("Peers: {:?}", node.config.peers);
    println!("==========================");

    // Clone values needed for background tasks
    let raft = Arc::clone(&node.raft);
    let config = node.config.clone();
    let store_for_tick = Arc::clone(&store);

    // Start Raft tick loop in background
    let raft_tick = Arc::clone(&raft);
    let config_tick = config.clone();
    tokio::spawn(async move {
        raft_tick_loop::run_raft_tick_loop(raft_tick, store_for_tick, config_tick).await;
    });

    // Start peer communication server in background
    let raft_peer = Arc::clone(&raft);
    let peer_addr = config.raft_addr();
    tokio::spawn(async move {
        peer_server::run_peer_server(&peer_addr, raft_peer).await;
    });

    // Run client server on main thread (handles SET/GET/DELETE from clients)
    server::run_with_cluster(&node.config.client_addr(), node).await;
}


