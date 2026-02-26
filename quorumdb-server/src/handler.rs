use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use quorumdb_core::{KvStore, Command};

use crate::protocol::{parse, Command as ProtocolCommand};
use crate::cluster_node::ClusterNode;

pub async fn handle_client(socket: TcpStream, engine: Arc<KvStore>) {
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();

        let bytes_read = match reader.read_line(&mut line).await {
            Ok(0) => return, // client closed connection
            Ok(n) => n,
            Err(_) => return,
        };

        if bytes_read == 0 {
            return;
        }

        let response = match parse(&line) {
            Ok(cmd) => execute(cmd, &engine).await,
            Err(err) => format!("ERROR: {}\n", err),
        };

        if writer.write_all(response.as_bytes()).await.is_err() {
            return;
        }
    }
}

/// Handle client with cluster node - routes writes through Raft consensus
pub async fn handle_client_with_cluster(socket: TcpStream, node: Arc<ClusterNode>) {
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();

        let bytes_read = match reader.read_line(&mut line).await {
            Ok(0) => return, // client closed connection
            Ok(n) => n,
            Err(_) => return,
        };

        if bytes_read == 0 {
            return;
        }

        let response = match parse(&line) {
            Ok(cmd) => execute_with_cluster(cmd, &node).await,
            Err(err) => format!("ERROR: {}\n", err),
        };

        if writer.write_all(response.as_bytes()).await.is_err() {
            return;
        }
    }
}

async fn execute(cmd: ProtocolCommand, engine: &KvStore) -> String {
    match cmd {
        ProtocolCommand::Set { key, value } => {
            match engine.set(key, value).await {
                Ok(_) => "OK\n".to_string(),
                Err(e) => format!("ERROR: {}\n", e),
            }
        }
        ProtocolCommand::Get { key } => {
            match engine.get(&key) {
                Ok(Some(val)) => format!("{}\n", val),
                Ok(None) => "NOT_FOUND\n".to_string(),
                Err(e) => format!("ERROR: {}\n", e),
            }
        }
        ProtocolCommand::Delete { key } => {
            match engine.delete(&key) {
                Ok(Some(_)) => "OK\n".to_string(),
                Ok(None) => "NOT_FOUND\n".to_string(),
                Err(e) => format!("ERROR: {}\n", e),
            }
        }
    }
}

/// Execute command through cluster node (with Raft consensus for writes)
async fn execute_with_cluster(cmd: ProtocolCommand, node: &ClusterNode) -> String {
    match cmd {
        ProtocolCommand::Get { key } => {
            // GET can be served by any node (read from local store)
            match node.store.get(&key) {
                Ok(Some(val)) => format!("{}\n", val),
                Ok(None) => "NOT_FOUND\n".to_string(),
                Err(e) => format!("ERROR: {}\n", e),
            }
        }
        ProtocolCommand::Set { key, value } => {
            // SET must go through Raft (only leader can accept)
            let mut raft = node.raft.write().await;
            if raft.is_leader() {
                let term = raft.current_term();
                raft.log.append(term, Command::Set { key: key.clone(), value: value.clone() });
                "QUEUED\n".to_string()
            } else {
                let leader = raft.current_leader.as_ref().cloned().unwrap_or_default();
                format!("NOT_LEADER:{}\n", leader)
            }
        }
        ProtocolCommand::Delete { key } => {
            // DELETE must go through Raft (only leader can accept)
            let mut raft = node.raft.write().await;
            if raft.is_leader() {
                let term = raft.current_term();
                raft.log.append(term, Command::Delete { key: key.clone() });
                "QUEUED\n".to_string()
            } else {
                let leader = raft.current_leader.as_ref().cloned().unwrap_or_default();
                format!("NOT_LEADER:{}\n", leader)
            }
        }
    }
}

