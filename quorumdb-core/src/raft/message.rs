use serde::{Deserialize, Serialize};

use crate::raft::log::LogEntry;

// =============================================================================
// Raft RPC Messages
// =============================================================================

/// All Raft RPC message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RaftMessage {
    // Leader election
    RequestVote(RequestVoteRequest),
    RequestVoteResponse(RequestVoteResponse),

    // Log replication & heartbeats
    AppendEntries(AppendEntriesRequest),
    AppendEntriesResponse(AppendEntriesResponse),

    // Client requests (forwarded to leader)
    ClientRequest(ClientRequest),
    ClientResponse(ClientResponse),
}

// =============================================================================
// Leader Election Messages
// =============================================================================

/// RequestVote RPC - sent by candidates to gather votes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteRequest {
    pub term: u64,
    pub candidate_id: String,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

impl RequestVoteRequest {
    pub fn new(
        term: u64,
        candidate_id: String,
        last_log_index: u64,
        last_log_term: u64,
    ) -> Self {
        Self {
            term,
            candidate_id,
            last_log_index,
            last_log_term,
        }
    }
}

/// RequestVote RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteResponse {
    pub term: u64,
    pub vote_granted: bool,
    pub voter_id: String,
}

impl RequestVoteResponse {
    pub fn granted(term: u64, voter_id: String) -> Self {
        Self {
            term,
            vote_granted: true,
            voter_id,
        }
    }

    pub fn denied(term: u64, voter_id: String) -> Self {
        Self {
            term,
            vote_granted: false,
            voter_id,
        }
    }
}

// =============================================================================
// Log Replication Messages
// =============================================================================

/// AppendEntries RPC - sent by leader for log replication & heartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    pub term: u64,
    pub leader_id: String,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u64,
}

impl AppendEntriesRequest {
    /// Create a heartbeat (empty entries)
    pub fn heartbeat(
        term: u64,
        leader_id: String,
        prev_log_index: u64,
        prev_log_term: u64,
        leader_commit: u64,
    ) -> Self {
        Self {
            term,
            leader_id,
            prev_log_index,
            prev_log_term,
            entries: Vec::new(),
            leader_commit,
        }
    }

    /// Create AppendEntries with log entries
    pub fn with_entries(
        term: u64,
        leader_id: String,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: Vec<LogEntry>,
        leader_commit: u64,
    ) -> Self {
        Self {
            term,
            leader_id,
            prev_log_index,
            prev_log_term,
            entries,
            leader_commit,
        }
    }

    pub fn is_heartbeat(&self) -> bool {
        self.entries.is_empty()
    }
}

/// AppendEntries RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    pub term: u64,
    pub success: bool,
    pub follower_id: String,
    pub match_index: u64,
}

impl AppendEntriesResponse {
    pub fn success(term: u64, follower_id: String, match_index: u64) -> Self {
        Self {
            term,
            success: true,
            follower_id,
            match_index,
        }
    }

    pub fn failure(term: u64, follower_id: String, match_index: u64) -> Self {
        Self {
            term,
            success: false,
            follower_id,
            match_index,
        }
    }
}

// =============================================================================
// Client Messages
// =============================================================================

/// Client request to the cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientRequest {
    pub request_id: String,
    pub command: ClientCommand,
}

/// Client commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientCommand {
    Set { key: String, value: String },
    Get { key: String },
    Delete { key: String },
}

/// Response to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientResponse {
    pub request_id: String,
    pub success: bool,
    pub result: Option<String>,
    pub leader_hint: Option<String>,
}

impl ClientResponse {
    pub fn success(request_id: String, result: Option<String>) -> Self {
        Self {
            request_id,
            success: true,
            result,
            leader_hint: None,
        }
    }

    pub fn not_leader(request_id: String, leader_hint: Option<String>) -> Self {
        Self {
            request_id,
            success: false,
            result: Some("Not the leader".to_string()),
            leader_hint,
        }
    }

    pub fn error(request_id: String, error: String) -> Self {
        Self {
            request_id,
            success: false,
            result: Some(error),
            leader_hint: None,
        }
    }
}
