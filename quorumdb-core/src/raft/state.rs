use std::time::{Duration, Instant};

/// The three states a Raft node can be in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Follower,
    Candidate,
    Leader,
}

impl Default for NodeState {
    fn default() -> Self {
        NodeState::Follower
    }
}

impl std::fmt::Display for NodeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeState::Follower => write!(f, "Follower"),
            NodeState::Candidate => write!(f, "Candidate"),
            NodeState::Leader => write!(f, "Leader"),
        }
    }
}

/// Persistent state - must be saved to stable storage before responding to RPCs
#[derive(Debug, Clone)]
pub struct PersistentState {
    pub current_term: u64,
    pub voted_for: Option<String>,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            current_term: 0,
            voted_for: None,
        }
    }
}

impl PersistentState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_term(&mut self, term: u64) {
        if term > self.current_term {
            self.current_term = term;
            self.voted_for = None;
        }
    }

    pub fn vote_for(&mut self, candidate_id: String) -> bool {
        if self.voted_for.is_none() {
            self.voted_for = Some(candidate_id);
            true
        } else {
            false
        }
    }

    pub fn already_voted_for(&self, candidate_id: &str) -> bool {
        self.voted_for.as_deref() == Some(candidate_id)
    }
}

/// Volatile state - can be reconstructed after crash
#[derive(Debug, Clone)]
pub struct VolatileState {
    pub commit_index: u64,
    pub last_applied: u64,
}

impl Default for VolatileState {
    fn default() -> Self {
        Self {
            commit_index: 0,
            last_applied: 0,
        }
    }
}

impl VolatileState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_commit_index(&mut self, new_index: u64) {
        if new_index > self.commit_index {
            self.commit_index = new_index;
        }
    }

    pub fn apply_entries(&mut self) -> Vec<u64> {
        let mut applied = Vec::new();
        while self.last_applied < self.commit_index {
            self.last_applied += 1;
            applied.push(self.last_applied);
        }
        applied
    }
}

/// Volatile state on leaders - reinitialized after election
#[derive(Debug, Clone)]
pub struct LeaderState {
    pub next_index: std::collections::HashMap<String, u64>,
    pub match_index: std::collections::HashMap<String, u64>,
}

impl LeaderState {
    pub fn new(peers: &[String], last_log_index: u64) -> Self {
        let mut next_index = std::collections::HashMap::new();
        let mut match_index = std::collections::HashMap::new();

        for peer in peers {
            next_index.insert(peer.clone(), last_log_index + 1);
            match_index.insert(peer.clone(), 0);
        }

        Self {
            next_index,
            match_index,
        }
    }

    pub fn update_peer_progress(&mut self, peer: &str, match_idx: u64) {
        if let Some(m) = self.match_index.get_mut(peer) {
            *m = match_idx;
        }
        if let Some(n) = self.next_index.get_mut(peer) {
            *n = match_idx + 1;
        }
    }

    pub fn decrement_next_index(&mut self, peer: &str) {
        if let Some(n) = self.next_index.get_mut(peer) {
            if *n > 1 {
                *n -= 1;
            }
        }
    }

    pub fn get_next_index(&self, peer: &str) -> u64 {
        self.next_index.get(peer).copied().unwrap_or(1)
    }

    pub fn get_match_index(&self, peer: &str) -> u64 {
        self.match_index.get(peer).copied().unwrap_or(0)
    }

    pub fn calculate_commit_index(&self, current_commit: u64, current_term: u64, log_term_at: impl Fn(u64) -> Option<u64>) -> u64 {
        let mut match_indices: Vec<u64> = self.match_index.values().copied().collect();
        match_indices.sort_unstable();

        if match_indices.is_empty() {
            return current_commit;
        }

        let majority_index = match_indices.len() / 2;
        let potential_commit = match_indices[majority_index];

        if potential_commit > current_commit {
            if let Some(term) = log_term_at(potential_commit) {
                if term == current_term {
                    return potential_commit;
                }
            }
        }

        current_commit
    }
}

/// Election timeout configuration
#[derive(Debug, Clone)]
pub struct ElectionConfig {
    pub min_timeout_ms: u64,
    pub max_timeout_ms: u64,
    pub heartbeat_interval_ms: u64,
}

impl Default for ElectionConfig {
    fn default() -> Self {
        Self {
            min_timeout_ms: 150,
            max_timeout_ms: 300,
            heartbeat_interval_ms: 50,
        }
    }
}

impl ElectionConfig {
    pub fn random_timeout(&self) -> Duration {
        use std::time::SystemTime;

        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64;

        let range = self.max_timeout_ms - self.min_timeout_ms;
        let timeout = self.min_timeout_ms + (nanos % range);

        Duration::from_millis(timeout)
    }

    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_millis(self.heartbeat_interval_ms)
    }
}

/// Tracks election timing
#[derive(Debug, Clone)]
pub struct ElectionTimer {
    pub last_reset: Instant,
    pub timeout: Duration,
    pub config: ElectionConfig,
}

impl ElectionTimer {
    pub fn new(config: ElectionConfig) -> Self {
        let timeout = config.random_timeout();
        Self {
            last_reset: Instant::now(),
            timeout,
            config,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.last_reset.elapsed() >= self.timeout
    }

    pub fn reset(&mut self) {
        self.last_reset = Instant::now();
        self.timeout = self.config.random_timeout();
    }

    pub fn remaining(&self) -> Duration {
        let elapsed = self.last_reset.elapsed();
        if elapsed >= self.timeout {
            Duration::ZERO
        } else {
            self.timeout - elapsed
        }
    }
}
