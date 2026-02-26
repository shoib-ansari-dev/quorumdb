use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use quorumdb_core::{
    RaftNode, KvStore, Command,
    raft::ClientCommand,
};

use crate::cluster_config::ClusterConfig;

/// Cluster node - wraps RaftNode with KvStore (state machine)
pub struct ClusterNode {
    /// Raft consensus engine
    pub raft: Arc<RwLock<RaftNode>>,

    /// Key-value store (state machine)
    pub store: Arc<KvStore>,

    /// Cluster configuration
    pub config: ClusterConfig,

    /// Peer connections (will be populated by server)
    pub peers: Arc<RwLock<HashMap<String, tokio::net::TcpStream>>>,
}

impl ClusterNode {
    /// Create a new cluster node
    pub async fn new(config: ClusterConfig, store: Arc<KvStore>) -> Self {
        let raft = RaftNode::new(
            config.node_id.clone(),
            config.peer_ids(),
        );

        Self {
            raft: Arc::new(RwLock::new(raft)),
            store,
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Process a client command - only leader can accept writes
    pub async fn handle_client_command(&self, cmd: ClientCommand, _request_id: String) -> String {
        match cmd {
            ClientCommand::Get { key } => {
                // GET can be served by any node (read-only)
                match self.store.get(&key) {
                    Ok(Some(value)) => value,
                    Ok(None) => "NOT_FOUND".to_string(),
                    Err(e) => format!("ERROR: {}", e),
                }
            }
            ClientCommand::Set { key, value } => {
                // Only leader can accept writes
                let mut raft = self.raft.write().await;
                if raft.is_leader() {
                    let term = raft.current_term();
                    raft.log.append(term, Command::Set { key, value });
                    "QUEUED".to_string()
                } else {
                    format!(
                        "NOT_LEADER:{}",
                        raft.current_leader.as_ref().cloned().unwrap_or_default()
                    )
                }
            }
            ClientCommand::Delete { key } => {
                // Only leader can accept deletes
                let mut raft = self.raft.write().await;
                if raft.is_leader() {
                    let term = raft.current_term();
                    raft.log.append(term, Command::Delete { key });
                    "QUEUED".to_string()
                } else {
                    format!(
                        "NOT_LEADER:{}",
                        raft.current_leader.as_ref().cloned().unwrap_or_default()
                    )
                }
            }
        }
    }

    /// Get node state for diagnostics
    pub async fn get_status(&self) -> String {
        let raft = self.raft.read().await;
        format!(
            "node={} state={:?} term={} leader={:?} committed={}",
            self.config.node_id,
            raft.state,
            raft.current_term(),
            raft.current_leader,
            raft.commit_index()
        )
    }
}

