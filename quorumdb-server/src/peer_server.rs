use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use quorumdb_core::{RaftNode, RaftMessage};
use crate::raft_handler::handle_raft_message;

/// Run the peer communication server - handles Raft RPC from other nodes
pub async fn run_peer_server(
    addr: &str,
    raft: Arc<RwLock<RaftNode>>,
) {
    let listener = TcpListener::bind(addr)
        .await
        .expect(&format!("Failed to bind peer server to {}", addr));

    println!("[PEER SERVER] Listening on {}", addr);

    loop {
        match listener.accept().await {
            Ok((socket, peer_addr)) => {
                let raft = Arc::clone(&raft);
                
                tokio::spawn(async move {
                    handle_peer_connection(socket, raft, peer_addr).await;
                });
            }
            Err(e) => {
                eprintln!("Failed to accept peer connection: {}", e);
            }
        }
    }
}

/// Handle a single peer connection
async fn handle_peer_connection(
    socket: tokio::net::TcpStream,
    raft: Arc<RwLock<RaftNode>>,
    peer_addr: std::net::SocketAddr,
) {
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();

        let bytes_read = match reader.read_line(&mut line).await {
            Ok(0) => return, // Connection closed
            Ok(n) => n,
            Err(e) => {
                eprintln!("Error reading from peer {}: {}", peer_addr, e);
                return;
            }
        };

        if bytes_read == 0 {
            return;
        }

        // Parse the Raft message
        let message: RaftMessage = match serde_json::from_str(line.trim()) {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("Invalid message from {}: {}", peer_addr, e);
                let _ = writer.write_all(b"ERROR: Invalid message format\n").await;
                continue;
            }
        };

        // Process the message
        let response = handle_raft_message(Arc::clone(&raft), message).await;

        // Send response
        if let Err(e) = writer.write_all(response.as_bytes()).await {
            eprintln!("Failed to send response to {}: {}", peer_addr, e);
            return;
        }
        if let Err(e) = writer.write_all(b"\n").await {
            eprintln!("Failed to send newline to {}: {}", peer_addr, e);
            return;
        }
    }
}

