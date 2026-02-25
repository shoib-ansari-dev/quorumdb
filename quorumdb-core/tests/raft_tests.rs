use quorumdb_core::raft::state::{
    NodeState, PersistentState, VolatileState, LeaderState, ElectionConfig, ElectionTimer
};
use quorumdb_core::raft::log::{RaftLog, LogEntry, Command};
use quorumdb_core::raft::message::{
    RequestVoteRequest, RequestVoteResponse,
    AppendEntriesRequest, AppendEntriesResponse,
    ClientResponse,
};
use quorumdb_core::raft::node::RaftNode;

// ==================== State Tests ====================

mod state_tests {
    use super::*;

    #[test]
    fn test_node_state_default() {
        assert_eq!(NodeState::default(), NodeState::Follower);
    }

    #[test]
    fn test_persistent_state_update_term() {
        let mut state = PersistentState::new();
        state.current_term = 5;
        state.voted_for = Some("node1".to_string());

        // Higher term should clear vote
        state.update_term(6);
        assert_eq!(state.current_term, 6);
        assert_eq!(state.voted_for, None);

        // Lower term should not update
        state.current_term = 10;
        state.voted_for = Some("node2".to_string());
        state.update_term(5);
        assert_eq!(state.current_term, 10);
        assert_eq!(state.voted_for, Some("node2".to_string()));
    }

    #[test]
    fn test_vote_for() {
        let mut state = PersistentState::new();

        // First vote should succeed
        assert!(state.vote_for("node1".to_string()));
        assert_eq!(state.voted_for, Some("node1".to_string()));

        // Second vote should fail
        assert!(!state.vote_for("node2".to_string()));
        assert_eq!(state.voted_for, Some("node1".to_string()));
    }

    #[test]
    fn test_volatile_state_apply_entries() {
        let mut state = VolatileState::new();
        state.commit_index = 5;

        let applied = state.apply_entries();
        assert_eq!(applied, vec![1, 2, 3, 4, 5]);
        assert_eq!(state.last_applied, 5);

        // No more entries to apply
        let applied2 = state.apply_entries();
        assert!(applied2.is_empty());
    }

    #[test]
    fn test_leader_state_peer_progress() {
        let peers = vec!["peer1".to_string(), "peer2".to_string()];
        let mut leader = LeaderState::new(&peers, 10);

        // Initial state
        assert_eq!(leader.get_next_index("peer1"), 11);
        assert_eq!(leader.get_match_index("peer1"), 0);

        // Update after successful append
        leader.update_peer_progress("peer1", 8);
        assert_eq!(leader.get_next_index("peer1"), 9);
        assert_eq!(leader.get_match_index("peer1"), 8);

        // Decrement after failed append
        leader.decrement_next_index("peer2");
        assert_eq!(leader.get_next_index("peer2"), 10);
    }

    #[test]
    fn test_election_timer() {
        let config = ElectionConfig {
            min_timeout_ms: 100,
            max_timeout_ms: 200,
            heartbeat_interval_ms: 50,
        };

        let timer = ElectionTimer::new(config);
        assert!(!timer.is_expired());
        assert!(timer.timeout.as_millis() >= 100);
        assert!(timer.timeout.as_millis() < 200);
    }
}

// ==================== Log Tests ====================

mod log_tests {
    use super::*;

    #[test]
    fn test_log_append() {
        let mut log = RaftLog::new();

        let idx1 = log.append(1, Command::Set { 
            key: "k1".to_string(), 
            value: "v1".to_string() 
        });
        assert_eq!(idx1, 1);
        assert_eq!(log.last_index(), 1);
        assert_eq!(log.last_term(), 1);

        let idx2 = log.append(1, Command::Set { 
            key: "k2".to_string(), 
            value: "v2".to_string() 
        });
        assert_eq!(idx2, 2);
        assert_eq!(log.last_index(), 2);
    }

    #[test]
    fn test_log_get() {
        let mut log = RaftLog::new();
        log.append(1, Command::Set { key: "k1".to_string(), value: "v1".to_string() });
        log.append(2, Command::Set { key: "k2".to_string(), value: "v2".to_string() });

        assert!(log.get(0).is_none());
        assert_eq!(log.get(1).unwrap().term, 1);
        assert_eq!(log.get(2).unwrap().term, 2);
        assert!(log.get(3).is_none());
    }

    #[test]
    fn test_log_term_at() {
        let mut log = RaftLog::new();
        log.append(1, Command::Noop);
        log.append(2, Command::Noop);
        log.append(2, Command::Noop);

        assert_eq!(log.term_at(0), Some(0));
        assert_eq!(log.term_at(1), Some(1));
        assert_eq!(log.term_at(2), Some(2));
        assert_eq!(log.term_at(3), Some(2));
        assert_eq!(log.term_at(4), None);
    }

    #[test]
    fn test_log_matches() {
        let mut log = RaftLog::new();
        log.append(1, Command::Noop);
        log.append(2, Command::Noop);

        // Empty case
        assert!(log.matches(0, 0));

        // Matching entries
        assert!(log.matches(1, 1));
        assert!(log.matches(2, 2));

        // Non-matching
        assert!(!log.matches(1, 2)); // Wrong term
        assert!(!log.matches(3, 2)); // Index doesn't exist
    }

    #[test]
    fn test_log_is_up_to_date() {
        let mut log = RaftLog::new();
        log.append(1, Command::Noop);
        log.append(2, Command::Noop);
        // Log: index=2, term=2

        // Higher term is more up-to-date
        assert!(log.is_up_to_date(3, 1));

        // Same term, higher index is more up-to-date
        assert!(log.is_up_to_date(2, 3));

        // Same term and index
        assert!(log.is_up_to_date(2, 2));

        // Lower term is less up-to-date
        assert!(!log.is_up_to_date(1, 5));
    }

    #[test]
    fn test_log_truncate() {
        let mut log = RaftLog::new();
        log.append(1, Command::Noop);
        log.append(2, Command::Noop);
        log.append(3, Command::Noop);

        log.truncate_after(1);

        assert_eq!(log.last_index(), 1);
        assert_eq!(log.last_term(), 1);
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_handle_append_entries() {
        let mut log = RaftLog::new();
        log.append(1, Command::Noop);
        log.append(1, Command::Noop);

        // Append new entries
        let new_entries = vec![
            LogEntry::new(2, 3, Command::Set { key: "k".to_string(), value: "v".to_string() }),
        ];

        assert!(log.handle_append_entries(2, 1, new_entries));
        assert_eq!(log.last_index(), 3);
        assert_eq!(log.last_term(), 2);
    }

    #[test]
    fn test_handle_append_entries_conflict() {
        let mut log = RaftLog::new();
        log.append(1, Command::Noop);
        log.append(1, Command::Noop); // Will conflict

        // Leader sends entry at index 2 with different term
        let new_entries = vec![
            LogEntry::new(2, 2, Command::Set { key: "k".to_string(), value: "v".to_string() }),
        ];

        assert!(log.handle_append_entries(1, 1, new_entries));
        assert_eq!(log.get(2).unwrap().term, 2); // Should be replaced
    }

    #[test]
    fn test_handle_append_entries_fails_consistency() {
        let mut log = RaftLog::new();
        log.append(1, Command::Noop);

        // Try to append with wrong prev_log_term
        let new_entries = vec![
            LogEntry::new(2, 2, Command::Noop),
        ];

        assert!(!log.handle_append_entries(1, 2, new_entries)); // Should fail
    }
}

// ==================== Message Tests ====================

mod message_tests {
    use super::*;

    #[test]
    fn test_request_vote_request() {
        let req = RequestVoteRequest::new(5, "node1".to_string(), 10, 4);
        assert_eq!(req.term, 5);
        assert_eq!(req.candidate_id, "node1");
        assert_eq!(req.last_log_index, 10);
        assert_eq!(req.last_log_term, 4);
    }

    #[test]
    fn test_request_vote_response() {
        let granted = RequestVoteResponse::granted(5, "voter1".to_string());
        assert!(granted.vote_granted);

        let denied = RequestVoteResponse::denied(6, "voter2".to_string());
        assert!(!denied.vote_granted);
        assert_eq!(denied.term, 6);
    }

    #[test]
    fn test_append_entries_heartbeat() {
        let hb = AppendEntriesRequest::heartbeat(5, "leader".to_string(), 10, 4, 8);
        assert!(hb.is_heartbeat());
        assert!(hb.entries.is_empty());
    }

    #[test]
    fn test_append_entries_with_entries() {
        let entries = vec![
            LogEntry::new(5, 11, Command::Set { key: "k".to_string(), value: "v".to_string() }),
        ];

        let req = AppendEntriesRequest::with_entries(
            5, "leader".to_string(), 10, 4, entries, 8
        );

        assert!(!req.is_heartbeat());
        assert_eq!(req.entries.len(), 1);
    }

    #[test]
    fn test_append_entries_response() {
        let success = AppendEntriesResponse::success(5, "follower1".to_string(), 15);
        assert!(success.success);
        assert_eq!(success.match_index, 15);

        let failure = AppendEntriesResponse::failure(5, "follower2".to_string(), 10);
        assert!(!failure.success);
    }

    #[test]
    fn test_client_response() {
        let success = ClientResponse::success("req1".to_string(), Some("value".to_string()));
        assert!(success.success);
        assert_eq!(success.result, Some("value".to_string()));

        let redirect = ClientResponse::not_leader("req2".to_string(), Some("node3".to_string()));
        assert!(!redirect.success);
        assert_eq!(redirect.leader_hint, Some("node3".to_string()));
    }
}

// ==================== Node Tests ====================

mod node_tests {
    use super::*;

    fn create_cluster() -> (RaftNode, RaftNode, RaftNode) {
        let node1 = RaftNode::new("node1".to_string(), vec!["node2".to_string(), "node3".to_string()]);
        let node2 = RaftNode::new("node2".to_string(), vec!["node1".to_string(), "node3".to_string()]);
        let node3 = RaftNode::new("node3".to_string(), vec!["node1".to_string(), "node2".to_string()]);
        (node1, node2, node3)
    }

    #[test]
    fn test_node_creation() {
        let node = RaftNode::new("node1".to_string(), vec!["node2".to_string(), "node3".to_string()]);

        assert_eq!(node.id, "node1");
        assert_eq!(node.peers.len(), 2);
        assert!(node.is_follower());
        assert_eq!(node.current_term(), 0);
        assert_eq!(node.cluster_size(), 3);
        assert_eq!(node.quorum_size(), 2);
    }

    #[test]
    fn test_start_election() {
        let mut node = RaftNode::new("node1".to_string(), vec!["node2".to_string(), "node3".to_string()]);

        node.start_election();

        assert!(node.is_candidate());
        assert_eq!(node.current_term(), 1);
        assert_eq!(node.persistent.voted_for, Some("node1".to_string()));
        assert!(node.votes_received.contains("node1"));
    }

    #[test]
    fn test_become_leader_with_majority() {
        let mut node = RaftNode::new("node1".to_string(), vec!["node2".to_string(), "node3".to_string()]);

        node.start_election();
        assert!(node.is_candidate());

        // Simulate receiving vote from node2
        let vote_response = RequestVoteResponse::granted(1, "node2".to_string());
        let became_leader = node.handle_request_vote_response(&vote_response);

        assert!(became_leader);
        assert!(node.is_leader());
        assert!(node.leader_state.is_some());
    }

    #[test]
    fn test_request_vote_higher_term_becomes_follower() {
        let mut node = RaftNode::new("node1".to_string(), vec!["node2".to_string()]);
        node.start_election(); // term = 1

        // Receive vote request with higher term
        let request = RequestVoteRequest::new(5, "node2".to_string(), 0, 0);
        let response = node.handle_request_vote(&request);

        assert!(node.is_follower());
        assert_eq!(node.current_term(), 5);
        assert!(response.vote_granted);
    }

    #[test]
    fn test_request_vote_older_log_rejected() {
        let mut node = RaftNode::new("node1".to_string(), vec!["node2".to_string()]);

        // Add some log entries
        node.log.append(1, Command::Noop);
        node.log.append(2, Command::Noop);

        // Candidate has older log
        let request = RequestVoteRequest::new(3, "node2".to_string(), 1, 1);
        let response = node.handle_request_vote(&request);

        assert!(!response.vote_granted);
    }

    #[test]
    fn test_append_entries_heartbeat() {
        let (mut node1, mut node2, _) = create_cluster();

        // node1 becomes leader
        node1.start_election();
        node1.become_leader();

        // Create heartbeat
        let heartbeat = node1.create_append_entries("node2").unwrap();

        // node2 receives heartbeat
        let response = node2.handle_append_entries(&heartbeat);

        assert!(response.success);
        assert_eq!(node2.current_leader, Some("node1".to_string()));
    }

    #[test]
    fn test_append_entries_with_log() {
        let (mut node1, mut node2, _) = create_cluster();

        // Setup: node1 becomes leader
        node1.start_election();
        node1.become_leader();

        // Leader appends entry
        node1.log.append(1, Command::Set { 
            key: "key".to_string(), 
            value: "value".to_string() 
        });

        // Send to follower
        let request = node1.create_append_entries("node2").unwrap();
        let response = node2.handle_append_entries(&request);

        assert!(response.success);
        assert_eq!(node2.log.last_index(), node1.log.last_index());
    }

    #[test]
    fn test_append_entries_stale_term_rejected() {
        let mut node = RaftNode::new("node1".to_string(), vec!["node2".to_string()]);
        node.persistent.current_term = 5;

        // Receive AppendEntries from old term
        let request = AppendEntriesRequest::heartbeat(3, "node2".to_string(), 0, 0, 0);
        let response = node.handle_append_entries(&request);

        assert!(!response.success);
        assert_eq!(response.term, 5);
    }

    #[test]
    fn test_candidate_steps_down_on_append_entries() {
        let mut node = RaftNode::new("node1".to_string(), vec!["node2".to_string()]);
        node.start_election(); // Becomes candidate

        // Receive AppendEntries from another leader
        let request = AppendEntriesRequest::heartbeat(1, "node2".to_string(), 0, 0, 0);
        let response = node.handle_append_entries(&request);

        assert!(response.success);
        assert!(node.is_follower());
    }

    #[test]
    fn test_commit_index_advancement() {
        let (mut node1, mut node2, _) = create_cluster();

        // Setup: node1 becomes leader
        node1.start_election();
        node1.become_leader();

        // Leader has entry at index 2 (index 1 is noop from become_leader)
        node1.log.append(1, Command::Set { 
            key: "k".to_string(), 
            value: "v".to_string() 
        });

        // Replicate to node2
        let request = node1.create_append_entries("node2").unwrap();
        let response = node2.handle_append_entries(&request);

        // Handle response
        node1.handle_append_entries_response(&response);

        // Now majority have the entries, commit index should advance
        assert!(node1.volatile.commit_index > 0);
    }

    #[test]
    fn test_entries_to_apply() {
        let mut node = RaftNode::new("node1".to_string(), vec!["node2".to_string()]);

        // Add some entries
        node.log.append(1, Command::Set { key: "k1".to_string(), value: "v1".to_string() });
        node.log.append(1, Command::Set { key: "k2".to_string(), value: "v2".to_string() });

        // Update commit index
        node.volatile.commit_index = 2;

        // Get entries to apply
        let entries = node.get_entries_to_apply();

        assert_eq!(entries.len(), 2);
        assert_eq!(node.volatile.last_applied, 2);
    }

    #[test]
    fn test_full_election_simulation() {
        let (mut node1, mut node2, mut node3) = create_cluster();

        // node1 starts election
        node1.start_election();

        // Create vote request
        let vote_request = node1.create_request_vote();

        // node2 and node3 receive request
        let response2 = node2.handle_request_vote(&vote_request);
        let response3 = node3.handle_request_vote(&vote_request);

        // Both should grant vote (empty logs, first request)
        assert!(response2.vote_granted);
        assert!(response3.vote_granted);

        // node1 processes responses
        node1.handle_request_vote_response(&response2);

        // After 2 votes (self + node2), should become leader (quorum=2)
        assert!(node1.is_leader());
    }
}

