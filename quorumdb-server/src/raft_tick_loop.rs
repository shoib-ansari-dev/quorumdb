use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use tokio::sync::RwLock;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use quorumdb_core::{RaftNode, KvStore, Command, RaftMessage};

use crate::cluster_config::ClusterConfig;

/// Run the Raft tick loop - drives elections, heartbeats, and log application
pub async fn run_raft_tick_loop(
    raft: Arc<RwLock<RaftNode>>,
    store: Arc<KvStore>,
    config: ClusterConfig,
) {
    let tick_interval = Duration::from_millis(10);
    let heartbeat_interval = Duration::from_millis(50);
    let mut last_heartbeat = std::time::Instant::now();

    // Build peer address map: node_id -> address
    let peer_map: HashMap<String, String> = config.peers.iter()
        .map(|(id, addr)| (id.clone(), addr.clone()))
        .collect();

    loop {
        // ==================== Check Election Timeout ====================
        {
            let should_start;
            let request;
            {
                let mut raft_guard = raft.write().await;
                should_start = raft_guard.should_start_election();
                if should_start {
                    println!("[{}] Election timeout - starting election for term {}",
                        raft_guard.id, raft_guard.current_term() + 1);
                    raft_guard.start_election();
                    request = Some(raft_guard.create_request_vote());
                } else {
                    request = None;
                }
            }

            // Send vote requests to all peers (outside lock)
            if let Some(req) = request {
                for (peer_id, peer_addr) in peer_map.iter() {
                    let msg = RaftMessage::RequestVote(req.clone());
                    let raft_clone = Arc::clone(&raft);
                    let peer_addr = peer_addr.clone();
                    let peer_id = peer_id.clone();

                    tokio::spawn(async move {
                        if let Err(e) = send_raft_message(&peer_addr, msg, raft_clone).await {
                            eprintln!("Failed to send RequestVote to {}: {}", peer_id, e);
                        }
                    });
                }
            }
        }

        // ==================== Leader: Send Heartbeats ====================
        if last_heartbeat.elapsed() >= heartbeat_interval {
            let heartbeats;
            {
                let raft_guard = raft.read().await;
                if raft_guard.is_leader() {
                    heartbeats = raft_guard.create_heartbeats();
                } else {
                    heartbeats = Vec::new();
                }
            }

            for (peer_id, request) in heartbeats {
                // Find peer address from map
                if let Some(peer_addr) = peer_map.get(&peer_id) {
                    let msg = RaftMessage::AppendEntries(request);
                    let raft_clone = Arc::clone(&raft);
                    let peer_addr = peer_addr.clone();
                    let peer_id = peer_id.clone();

                    tokio::spawn(async move {
                        if let Err(e) = send_raft_message(&peer_addr, msg, raft_clone).await {
                            // Heartbeat failures are normal during partitions
                            eprintln!("Heartbeat to {} failed: {}", peer_id, e);
                        }
                    });
                }
            }

            last_heartbeat = std::time::Instant::now();
        }

        // ==================== Apply Committed Entries ====================
        {
            let entries;
            {
                let mut raft_guard = raft.write().await;
                if raft_guard.has_entries_to_apply() {
                    entries = raft_guard.get_entries_to_apply();
                } else {
                    entries = Vec::new();
                }
            }

            for entry in entries {
                match &entry.command {
                    Command::Set { key, value } => {
                        if let Err(e) = store.set(key.clone(), value.clone()).await {
                            eprintln!("Failed to apply SET {}: {}", key, e);
                        } else {
                            println!("[APPLY] SET {} = {}", key, value);
                        }
                    }
                    Command::Delete { key } => {
                        if let Err(e) = store.delete(key) {
                            eprintln!("Failed to apply DELETE {}: {}", key, e);
                        } else {
                            println!("[APPLY] DELETE {}", key);
                        }
                    }
                    Command::Noop => {
                        // No-op entries are used for leader establishment
                    }
                }
            }
        }

        tokio::time::sleep(tick_interval).await;
    }
}

/// Send a Raft message to a peer and handle the response
async fn send_raft_message(
    peer_addr: &str,
    message: RaftMessage,
    raft: Arc<RwLock<RaftNode>>,
) -> Result<(), String> {
    // Connect to peer
    let stream = TcpStream::connect(peer_addr)
        .await
        .map_err(|e| format!("Connection failed: {}", e))?;

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Serialize and send message
    let json = serde_json::to_string(&message)
        .map_err(|e| format!("Serialization failed: {}", e))?;

    writer.write_all(json.as_bytes())
        .await
        .map_err(|e| format!("Write failed: {}", e))?;
    writer.write_all(b"\n")
        .await
        .map_err(|e| format!("Write newline failed: {}", e))?;

    // Read response
    let mut response_line = String::new();
    reader.read_line(&mut response_line)
        .await
        .map_err(|e| format!("Read failed: {}", e))?;

    // Parse and handle response
    if response_line.trim().is_empty() {
        return Ok(());
    }

    // Handle response based on message type
    match &message {
        RaftMessage::RequestVote(_) => {
            if let Ok(response) = serde_json::from_str::<quorumdb_core::RequestVoteResponse>(&response_line) {
                let mut raft = raft.write().await;
                let became_leader = raft.handle_request_vote_response(&response);
                if became_leader {
                    println!("[{}] Became LEADER for term {}", raft.id, raft.current_term());
                }
            }
        }
        RaftMessage::AppendEntries(_) => {
            if let Ok(response) = serde_json::from_str::<quorumdb_core::AppendEntriesResponse>(&response_line) {
                let mut raft = raft.write().await;
                raft.handle_append_entries_response(&response);
            }
        }
        _ => {}
    }

    Ok(())
}

