mod storage;
pub mod raft;

pub use storage::engine::StorageEngine;

// Type alias for the concrete implementation (String key-value store)
pub type KvStore = StorageEngine<String, String>;

// Re-export raft types
pub use raft::{RaftNode, NodeState, RaftLog, LogEntry, Command};
pub use raft::message::{
    RaftMessage, RequestVoteRequest, RequestVoteResponse,
    AppendEntriesRequest, AppendEntriesResponse,
    ClientRequest, ClientResponse, ClientCommand,
};


