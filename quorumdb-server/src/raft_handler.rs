use std::sync::Arc;
use tokio::sync::RwLock;
use quorumdb_core::{
    RaftNode, Command,
    raft::RaftMessage,
};
use serde_json;

/// Handle incoming Raft RPC messages
pub async fn handle_raft_message(
    raft: Arc<RwLock<RaftNode>>,
    message: RaftMessage,
) -> String {
    match message {
        // ============ ELECTION MESSAGES ============
        RaftMessage::RequestVote(req) => {
            let mut raft = raft.write().await;
            let response = raft.handle_request_vote(&req);
            serde_json::to_string(&response).unwrap_or_default()
        }

        RaftMessage::RequestVoteResponse(resp) => {
            let mut raft = raft.write().await;
            raft.handle_request_vote_response(&resp);
            "OK".to_string()
        }

        // ============ LOG REPLICATION MESSAGES ============
        RaftMessage::AppendEntries(req) => {
            let mut raft = raft.write().await;
            let response = raft.handle_append_entries(&req);
            serde_json::to_string(&response).unwrap_or_default()
        }

        RaftMessage::AppendEntriesResponse(resp) => {
            let mut raft = raft.write().await;
            raft.handle_append_entries_response(&resp);
            "OK".to_string()
        }

        // ============ CLIENT MESSAGES ============
        RaftMessage::ClientRequest(req) => {
            let mut raft = raft.write().await;
            let response = raft.handle_client_request(&req);
            serde_json::to_string(&response).unwrap_or_default()
        }

        RaftMessage::ClientResponse(_) => {
            // Shouldn't receive this on server
            "ERROR: Invalid message".to_string()
        }
    }
}

/// Apply committed entries to state machine
pub async fn apply_committed_entries(
    raft: Arc<RwLock<RaftNode>>,
    apply_fn: impl Fn(Command),
) {
    let mut raft = raft.write().await;
    for entry in raft.get_entries_to_apply() {
        apply_fn(entry.command);
    }
}

