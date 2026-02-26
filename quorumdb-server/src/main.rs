use std::sync::Arc;
use quorumdb_core::KvStore;
use quorumdb_server::cluster_config::ClusterConfig;
use quorumdb_server::cluster_node::ClusterNode;

mod server;
mod protocol;
mod handler;
mod cluster_config;
mod cluster_node;
mod raft_handler;

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

    // TODO: Start Raft tick loop in background
    // TODO: Start peer communication server
    // TODO: Keep existing client server

    // For now, just run the client server
    server::run(&node.config.client_addr(), store).await;
}


