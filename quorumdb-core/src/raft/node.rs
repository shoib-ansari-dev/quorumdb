use std::collections::HashSet;
use crate::raft::state::{NodeState, PersistentState, VolatileState, LeaderState, ElectionConfig, ElectionTimer};
use crate::raft::log::{RaftLog, Command, LogEntry};
use crate::raft::message::{
    RequestVoteRequest, RequestVoteResponse,
    AppendEntriesRequest, AppendEntriesResponse,
    ClientRequest, ClientResponse, ClientCommand,
};

/// The main Raft node implementation
#[derive(Debug)]
pub struct RaftNode {
    pub id: String,
    pub peers: Vec<String>,
    pub state: NodeState,
    pub persistent: PersistentState,
    pub volatile: VolatileState,
    pub log: RaftLog,
    pub leader_state: Option<LeaderState>,
    pub current_leader: Option<String>,
    pub election_timer: ElectionTimer,
    pub votes_received: HashSet<String>,
}

impl RaftNode {
    /// Create a new Raft node
    pub fn new(id: String, peers: Vec<String>) -> Self {
        let config = ElectionConfig::default();

        Self {
            id,
            peers,
            state: NodeState::Follower,
            persistent: PersistentState::new(),
            volatile: VolatileState::new(),
            log: RaftLog::new(),
            leader_state: None,
            current_leader: None,
            election_timer: ElectionTimer::new(config),
            votes_received: HashSet::new(),
        }
    }

    /// Create with custom election config
    pub fn with_config(id: String, peers: Vec<String>, config: ElectionConfig) -> Self {
        Self {
            id,
            peers,
            state: NodeState::Follower,
            persistent: PersistentState::new(),
            volatile: VolatileState::new(),
            log: RaftLog::new(),
            leader_state: None,
            current_leader: None,
            election_timer: ElectionTimer::new(config),
            votes_received: HashSet::new(),
        }
    }

    // ==================== State Queries ====================

    pub fn is_leader(&self) -> bool {
        self.state == NodeState::Leader
    }

    pub fn is_candidate(&self) -> bool {
        self.state == NodeState::Candidate
    }

    pub fn is_follower(&self) -> bool {
        self.state == NodeState::Follower
    }

    pub fn current_term(&self) -> u64 {
        self.persistent.current_term
    }

    pub fn commit_index(&self) -> u64 {
        self.volatile.commit_index
    }

    pub fn cluster_size(&self) -> usize {
        self.peers.len() + 1
    }

    pub fn quorum_size(&self) -> usize {
        (self.cluster_size() / 2) + 1
    }

    // ==================== State Transitions ====================

    pub fn become_follower(&mut self, term: u64) {
        self.state = NodeState::Follower;
        self.persistent.update_term(term);
        self.leader_state = None;
        self.votes_received.clear();
        self.election_timer.reset();
    }

    pub fn start_election(&mut self) {
        // Increment term
        self.persistent.current_term += 1;
        self.persistent.voted_for = Some(self.id.clone());

        // Transition to candidate
        self.state = NodeState::Candidate;
        self.leader_state = None;
        self.current_leader = None;

        // Vote for self
        self.votes_received.clear();
        self.votes_received.insert(self.id.clone());

        // Reset election timer
        self.election_timer.reset();
    }

    /// Get RequestVote request to send to peers
    pub fn create_request_vote(&self) -> RequestVoteRequest {
        RequestVoteRequest::new(
            self.persistent.current_term,
            self.id.clone(),
            self.log.last_index(),
            self.log.last_term(),
        )
    }

    /// Transition to leader state
    pub fn become_leader(&mut self) {
        self.state = NodeState::Leader;
        self.current_leader = Some(self.id.clone());

        // Initialize leader state
        self.leader_state = Some(LeaderState::new(&self.peers, self.log.last_index()));

        self.votes_received.clear();

        // Append a no-op entry to establish leadership (Raft optimization)
        // This ensures we can commit entries from previous terms
        self.log.append(self.persistent.current_term, Command::Noop);
    }

    // ==================== Election Handling ====================

    pub fn should_start_election(&self) -> bool {
        self.state != NodeState::Leader && self.election_timer.is_expired()
    }

    /// Handle RequestVote RPC
    pub fn handle_request_vote(&mut self, request: &RequestVoteRequest) -> RequestVoteResponse {
        // Rule 1: Reply false if term < currentTerm
        if request.term < self.persistent.current_term {
            return RequestVoteResponse::denied(
                self.persistent.current_term,
                self.id.clone(),
            );
        }

        // Rule 2: If term > currentTerm, update term and become follower
        if request.term > self.persistent.current_term {
            self.become_follower(request.term);
        }

        // Rule 3: Check if we can vote for this candidate
        let can_vote = match &self.persistent.voted_for {
            None => true,
            Some(voted_for) => voted_for == &request.candidate_id,
        };

        // Rule 4: Check if candidate's log is at least as up-to-date
        let log_ok = self.log.is_up_to_date(request.last_log_term, request.last_log_index);

        if can_vote && log_ok {
            // Grant vote
            self.persistent.voted_for = Some(request.candidate_id.clone());
            self.election_timer.reset();

            RequestVoteResponse::granted(self.persistent.current_term, self.id.clone())
        } else {
            RequestVoteResponse::denied(self.persistent.current_term, self.id.clone())
        }
    }

    /// Handle RequestVote response (as candidate)
    pub fn handle_request_vote_response(&mut self, response: &RequestVoteResponse) -> bool {
        if response.term > self.persistent.current_term {
            self.become_follower(response.term);
            return false;
        }

        if self.state != NodeState::Candidate {
            return false;
        }

        if response.term != self.persistent.current_term {
            return false;
        }

        if response.vote_granted {
            self.votes_received.insert(response.voter_id.clone());

            if self.votes_received.len() >= self.quorum_size() {
                self.become_leader();
                return true;
            }
        }

        false
    }

    // ==================== Log Replication ====================

    /// Create AppendEntries request for a peer (as leader)
    pub fn create_append_entries(&self, peer: &str) -> Option<AppendEntriesRequest> {
        if self.state != NodeState::Leader {
            return None;
        }

        let leader_state = self.leader_state.as_ref()?;
        let next_index = leader_state.get_next_index(peer);

        let prev_log_index = if next_index > 0 { next_index - 1 } else { 0 };
        let prev_log_term = self.log.term_at(prev_log_index).unwrap_or(0);

        let entries = self.log.get_from(next_index);

        Some(AppendEntriesRequest::with_entries(
            self.persistent.current_term,
            self.id.clone(),
            prev_log_index,
            prev_log_term,
            entries,
            self.volatile.commit_index,
        ))
    }

    /// Create heartbeat for all peers
    pub fn create_heartbeats(&self) -> Vec<(String, AppendEntriesRequest)> {
        if self.state != NodeState::Leader {
            return Vec::new();
        }

        self.peers.iter()
            .filter_map(|peer| {
                self.create_append_entries(peer)
                    .map(|req| (peer.clone(), req))
            })
            .collect()
    }

    /// Handle AppendEntries RPC (as follower)
    pub fn handle_append_entries(&mut self, request: &AppendEntriesRequest) -> AppendEntriesResponse {
        // Rule 1: Reply false if term < currentTerm
        if request.term < self.persistent.current_term {
            return AppendEntriesResponse::failure(
                self.persistent.current_term,
                self.id.clone(),
                self.log.last_index(),
            );
        }

        // Valid leader - reset election timer and update leader
        self.election_timer.reset();
        self.current_leader = Some(request.leader_id.clone());

        // If term > currentTerm, become follower
        if request.term > self.persistent.current_term {
            self.become_follower(request.term);
        } else if self.state == NodeState::Candidate {
            // If we're a candidate and receive AppendEntries from valid leader
            self.become_follower(request.term);
        }

        // Rule 2 & 3: Check log consistency and append entries
        let success = self.log.handle_append_entries(
            request.prev_log_index,
            request.prev_log_term,
            request.entries.clone(),
        );

        if !success {
            return AppendEntriesResponse::failure(
                self.persistent.current_term,
                self.id.clone(),
                self.log.last_index(),
            );
        }

        // Rule 4: Update commit index
        if request.leader_commit > self.volatile.commit_index {
            self.volatile.commit_index = std::cmp::min(
                request.leader_commit,
                self.log.last_index(),
            );
        }

        AppendEntriesResponse::success(
            self.persistent.current_term,
            self.id.clone(),
            self.log.last_index(),
        )
    }

    /// Handle AppendEntries response (as leader)
    pub fn handle_append_entries_response(&mut self, response: &AppendEntriesResponse) {
        if response.term > self.persistent.current_term {
            self.become_follower(response.term);
            return;
        }

        if self.state != NodeState::Leader {
            return;
        }

        let leader_state = match self.leader_state.as_mut() {
            Some(ls) => ls,
            None => return,
        };

        if response.success {
            leader_state.update_peer_progress(&response.follower_id, response.match_index);

            self.try_advance_commit_index();
        } else {
            leader_state.decrement_next_index(&response.follower_id);
        }
    }

    /// Try to advance commit index based on replication progress
    fn try_advance_commit_index(&mut self) {
        if self.state != NodeState::Leader {
            return;
        }

        let leader_state = match self.leader_state.as_ref() {
            Some(ls) => ls,
            None => return,
        };

        let mut match_indices: Vec<u64> = leader_state.match_index.values().copied().collect();
        match_indices.push(self.log.last_index()); // Leader's own log
        match_indices.sort_unstable();

        let majority_idx = match_indices.len() - self.quorum_size();
        let potential_commit = match_indices[majority_idx];

        if potential_commit > self.volatile.commit_index {
            if let Some(term) = self.log.term_at(potential_commit) {
                if term == self.persistent.current_term {
                    self.volatile.commit_index = potential_commit;
                }
            }
        }
    }

    // ==================== Client Request Handling ====================

    /// Handle a client request
    pub fn handle_client_request(&mut self, request: &ClientRequest) -> ClientResponse {
        if self.state != NodeState::Leader {
            return ClientResponse::not_leader(
                request.request_id.clone(),
                self.current_leader.clone(),
            );
        }

        match &request.command {
            ClientCommand::Get { key: _ } => {
                ClientResponse::success(request.request_id.clone(), None)
            }
            ClientCommand::Set { key, value } => {
                let _index = self.log.append(
                    self.persistent.current_term,
                    Command::Set { key: key.clone(), value: value.clone() },
                );

                ClientResponse::success(request.request_id.clone(), None)
            }
            ClientCommand::Delete { key } => {
                let _index = self.log.append(
                    self.persistent.current_term,
                    Command::Delete { key: key.clone() },
                );

                ClientResponse::success(request.request_id.clone(), None)
            }
        }
    }

    // ==================== State Machine Application ====================

    pub fn get_entries_to_apply(&mut self) -> Vec<LogEntry> {
        let indices = self.volatile.apply_entries();

        indices.iter()
            .filter_map(|&idx| self.log.get(idx).cloned())
            .collect()
    }

    pub fn has_entries_to_apply(&self) -> bool {
        self.volatile.last_applied < self.volatile.commit_index
    }
}
