use std::sync::Arc;
use tokio::net::TcpListener;

use quorumdb_core::KvStore;

use crate::handler::{handle_client, handle_client_with_cluster};
use crate::cluster_node::ClusterNode;

pub async fn run(addr: &str, engine: Arc<KvStore>) {

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    loop{
        let (socket, _ )= listener
            .accept()
            .await
            .expect("Failed to accept connection");
        let engine= Arc::clone(&engine);

        tokio::spawn(async move {
            handle_client(socket, engine).await;
        });
    }
}

/// Run server with cluster node - routes writes through Raft consensus
pub async fn run_with_cluster(addr: &str, cluster_node: Arc<ClusterNode>) {
    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind client server");

    println!("[CLIENT SERVER] Listening on {}", addr);

    loop {
        let (socket, _) = listener
            .accept()
            .await
            .expect("Failed to accept connection");
        let node = Arc::clone(&cluster_node);

        tokio::spawn(async move {
            handle_client_with_cluster(socket, node).await;
        });
    }
}
