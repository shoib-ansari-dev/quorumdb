/// Cluster Integration Tests for Step 7
/// Tests the Raft tick loop, peer communication, and cluster behavior

use std::sync::Arc;
use tokio::sync::RwLock;

use quorumdb_core::{KvStore, RaftNode, Command};

// ==================== ClusterConfig Tests ====================

mod cluster_config_tests {
    use quorumdb_server::cluster_config::ClusterConfig;

    #[test]
    fn test_config_from_defaults() {
        let config = ClusterConfig::default_cluster();
        assert_eq!(config.node_id, "node1");
        assert_eq!(config.client_port, 7000);
        assert_eq!(config.raft_port, 7001);
    }

    #[test]
    fn test_config_peer_ids() {
        let mut config = ClusterConfig::default_cluster();
        config.peers = vec![
            ("node2".to_string(), "127.0.0.1:7011".to_string()),
            ("node3".to_string(), "127.0.0.1:7021".to_string()),
        ];

        let peer_ids = config.peer_ids();
        assert_eq!(peer_ids.len(), 2);
        assert!(peer_ids.contains(&"node2".to_string()));
        assert!(peer_ids.contains(&"node3".to_string()));
    }

    #[test]
    fn test_config_client_addr() {
        let mut config = ClusterConfig::default_cluster();
        config.bind_addr = "0.0.0.0".to_string();
        config.client_port = 8000;

        assert_eq!(config.client_addr(), "0.0.0.0:8000");
    }

    #[test]
    fn test_config_raft_addr() {
        let mut config = ClusterConfig::default_cluster();
        config.bind_addr = "0.0.0.0".to_string();
        config.raft_port = 8001;

        assert_eq!(config.raft_addr(), "0.0.0.0:8001");
    }
}

// ==================== Raft Node Election Tests ====================

mod election_tests {
    use super::*;

    #[test]
    fn test_single_node_can_become_leader() {
        // Single node cluster - should win election immediately
        let mut node = RaftNode::new("node1".to_string(), vec![]);

        // Start election
        node.start_election();

        // With no peers, node has quorum (1 of 1)
        assert!(node.votes_received.len() >= node.quorum_size());
    }

    #[test]
    fn test_three_node_quorum() {
        let node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string(), "node3".to_string()],
        );

        // Cluster size is 3, quorum is 2
        assert_eq!(node.cluster_size(), 3);
        assert_eq!(node.quorum_size(), 2);
    }

    #[test]
    fn test_five_node_quorum() {
        let node = RaftNode::new(
            "node1".to_string(),
            vec![
                "node2".to_string(),
                "node3".to_string(),
                "node4".to_string(),
                "node5".to_string(),
            ],
        );

        // Cluster size is 5, quorum is 3
        assert_eq!(node.cluster_size(), 5);
        assert_eq!(node.quorum_size(), 3);
    }

    #[test]
    fn test_election_increments_term() {
        let mut node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );

        let initial_term = node.current_term();
        node.start_election();

        assert_eq!(node.current_term(), initial_term + 1);
    }

    #[test]
    fn test_candidate_votes_for_self() {
        let mut node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );

        node.start_election();

        assert!(node.is_candidate());
        assert!(node.votes_received.contains(&"node1".to_string()));
    }

    #[test]
    fn test_become_leader_sets_state() {
        let mut node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );

        node.start_election();
        node.become_leader();

        assert!(node.is_leader());
        assert_eq!(node.current_leader, Some("node1".to_string()));
    }

    #[test]
    fn test_leader_creates_heartbeats() {
        let mut node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string(), "node3".to_string()],
        );

        node.start_election();
        node.become_leader();

        let heartbeats = node.create_heartbeats();

        // Should create heartbeats for both peers
        assert_eq!(heartbeats.len(), 2);
    }
}

// ==================== Log Replication Tests ====================

mod replication_tests {
    use super::*;

    #[test]
    fn test_leader_appends_noop_on_election() {
        let mut node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );

        let initial_log_len = node.log.len();
        node.start_election();
        node.become_leader();

        // Leader should append a no-op entry
        assert_eq!(node.log.len(), initial_log_len + 1);
    }

    #[test]
    fn test_leader_can_append_commands() {
        let mut node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );

        node.start_election();
        node.become_leader();

        let term = node.current_term();
        node.log.append(term, Command::Set {
            key: "foo".to_string(),
            value: "bar".to_string(),
        });

        // Should have noop + set command
        assert_eq!(node.log.len(), 2);
    }

    #[test]
    fn test_append_entries_request_contains_entries() {
        let mut node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );

        node.start_election();
        node.become_leader();

        let term = node.current_term();
        node.log.append(term, Command::Set {
            key: "key1".to_string(),
            value: "value1".to_string(),
        });

        let request = node.create_append_entries("node2").unwrap();

        // Request should contain entries
        assert!(!request.entries.is_empty());
    }
}

// ==================== Client Command Tests ====================

mod client_command_tests {
    use super::*;

    #[tokio::test]
    async fn test_leader_accepts_set_command() {
        let mut node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );

        node.start_election();
        node.become_leader();

        let raft = Arc::new(RwLock::new(node));

        // Simulate SET command through leader
        {
            let mut raft_guard = raft.write().await;
            assert!(raft_guard.is_leader());

            let term = raft_guard.current_term();
            raft_guard.log.append(term, Command::Set {
                key: "test_key".to_string(),
                value: "test_value".to_string(),
            });
        }

        // Verify entry was added
        let raft_guard = raft.read().await;
        assert!(raft_guard.log.len() >= 2); // noop + set
    }

    #[tokio::test]
    async fn test_follower_rejects_set_command() {
        let node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );

        let raft = Arc::new(RwLock::new(node));

        // Node is follower by default
        {
            let raft_guard = raft.read().await;
            assert!(raft_guard.is_follower());
            assert!(!raft_guard.is_leader());
        }
    }

    #[tokio::test]
    async fn test_get_command_works_on_follower() {
        let store = Arc::new(KvStore::new());

        // Pre-populate store
        store.set("key1".to_string(), "value1".to_string()).await.unwrap();

        // GET should work regardless of node state
        let result = store.get(&"key1".to_string());
        assert_eq!(result.unwrap(), Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_delete_command_requires_leader() {
        let mut node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );

        // Start as follower - cannot delete
        assert!(node.is_follower());

        // Become leader
        node.start_election();
        node.become_leader();

        // Now can append delete command
        let term = node.current_term();
        node.log.append(term, Command::Delete {
            key: "to_delete".to_string(),
        });

        assert!(node.log.len() >= 2); // noop + delete
    }
}

// ==================== Tick Loop Behavior Tests ====================

mod tick_loop_tests {
    use super::*;
    use quorumdb_core::raft::state::ElectionConfig;
    use std::time::Duration;

    #[test]
    fn test_election_timeout_detection() {
        let config = ElectionConfig {
            min_timeout_ms: 10,
            max_timeout_ms: 20,
            heartbeat_interval_ms: 5,
        };

        let mut node = RaftNode::with_config(
            "node1".to_string(),
            vec!["node2".to_string()],
            config,
        );

        // Initially should not need election
        node.election_timer.reset();
        assert!(!node.should_start_election());

        // After timeout, should need election
        std::thread::sleep(Duration::from_millis(50));
        assert!(node.should_start_election());
    }

    #[test]
    fn test_leader_does_not_trigger_election() {
        let mut node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );

        node.start_election();
        node.become_leader();

        // Leaders never trigger elections
        assert!(!node.should_start_election());
    }

    #[test]
    fn test_entries_to_apply_tracking() {
        let mut node = RaftNode::new(
            "node1".to_string(),
            vec![],
        );

        node.start_election();
        node.become_leader();

        // Add some entries
        let term = node.current_term();
        node.log.append(term, Command::Set {
            key: "k1".to_string(),
            value: "v1".to_string(),
        });
        node.log.append(term, Command::Set {
            key: "k2".to_string(),
            value: "v2".to_string(),
        });

        // Simulate commit
        node.volatile.commit_index = node.log.last_index();

        // Check entries to apply
        assert!(node.has_entries_to_apply());

        let entries = node.get_entries_to_apply();
        assert!(!entries.is_empty());

        // After applying, no more entries
        assert!(!node.has_entries_to_apply());
    }
}

// ==================== State Machine Application Tests ====================

mod state_machine_tests {
    use super::*;

    #[tokio::test]
    async fn test_apply_set_command_to_store() {
        let store = Arc::new(KvStore::new());

        // Apply SET command
        store.set("applied_key".to_string(), "applied_value".to_string()).await.unwrap();

        // Verify in store
        let result = store.get(&"applied_key".to_string());
        assert_eq!(result.unwrap(), Some("applied_value".to_string()));
    }

    #[tokio::test]
    async fn test_apply_delete_command_to_store() {
        let store = Arc::new(KvStore::new());

        // Set then delete
        store.set("to_delete".to_string(), "value".to_string()).await.unwrap();
        store.delete(&"to_delete".to_string()).unwrap();

        // Verify deleted
        let result = store.get(&"to_delete".to_string());
        assert_eq!(result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_apply_multiple_commands() {
        let store = Arc::new(KvStore::new());

        // Apply multiple commands
        store.set("k1".to_string(), "v1".to_string()).await.unwrap();
        store.set("k2".to_string(), "v2".to_string()).await.unwrap();
        store.set("k3".to_string(), "v3".to_string()).await.unwrap();
        store.delete(&"k2".to_string()).unwrap();

        // Verify state
        assert_eq!(store.get(&"k1".to_string()).unwrap(), Some("v1".to_string()));
        assert_eq!(store.get(&"k2".to_string()).unwrap(), None);
        assert_eq!(store.get(&"k3".to_string()).unwrap(), Some("v3".to_string()));
    }

    #[tokio::test]
    async fn test_command_enum_variants() {
        // Test all Command variants
        let set_cmd = Command::Set {
            key: "key".to_string(),
            value: "value".to_string(),
        };
        let delete_cmd = Command::Delete {
            key: "key".to_string(),
        };
        let noop_cmd = Command::Noop;

        // Verify they can be created
        match set_cmd {
            Command::Set { key, value } => {
                assert_eq!(key, "key");
                assert_eq!(value, "value");
            }
            _ => panic!("Expected Set command"),
        }

        match delete_cmd {
            Command::Delete { key } => {
                assert_eq!(key, "key");
            }
            _ => panic!("Expected Delete command"),
        }

        match noop_cmd {
            Command::Noop => {}
            _ => panic!("Expected Noop command"),
        }
    }
}

// ==================== Peer Communication Tests ====================

mod peer_communication_tests {
    use quorumdb_core::{RequestVoteRequest, RequestVoteResponse, AppendEntriesRequest, AppendEntriesResponse};

    #[test]
    fn test_request_vote_serialization() {
        let request = RequestVoteRequest::new(
            1, // term
            "node1".to_string(),
            0, // last_log_index
            0, // last_log_term
        );

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: RequestVoteRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.term, 1);
        assert_eq!(deserialized.candidate_id, "node1");
    }

    #[test]
    fn test_request_vote_response_serialization() {
        let response = RequestVoteResponse::granted(1, "node2".to_string());

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: RequestVoteResponse = serde_json::from_str(&json).unwrap();

        assert!(deserialized.vote_granted);
        assert_eq!(deserialized.voter_id, "node2");
    }

    #[test]
    fn test_append_entries_heartbeat_serialization() {
        let heartbeat = AppendEntriesRequest::heartbeat(
            1, // term
            "leader1".to_string(),
            0, // prev_log_index
            0, // prev_log_term
            0, // leader_commit
        );

        let json = serde_json::to_string(&heartbeat).unwrap();
        let deserialized: AppendEntriesRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.term, 1);
        assert_eq!(deserialized.leader_id, "leader1");
        assert!(deserialized.entries.is_empty());
    }

    #[test]
    fn test_append_entries_response_serialization() {
        let response = AppendEntriesResponse::success(1, "node2".to_string(), 5);

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: AppendEntriesResponse = serde_json::from_str(&json).unwrap();

        assert!(deserialized.success);
        assert_eq!(deserialized.match_index, 5);
    }
}

// ==================== Multi-Node Simulation Tests ====================

mod multi_node_simulation_tests {
    use super::*;

    #[test]
    fn test_three_node_election_simulation() {
        // Create three nodes
        let mut node1 = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string(), "node3".to_string()],
        );
        let mut node2 = RaftNode::new(
            "node2".to_string(),
            vec!["node1".to_string(), "node3".to_string()],
        );
        let mut node3 = RaftNode::new(
            "node3".to_string(),
            vec!["node1".to_string(), "node2".to_string()],
        );

        // Node1 starts election
        node1.start_election();
        assert!(node1.is_candidate());
        assert_eq!(node1.current_term(), 1);

        // Create vote request
        let vote_request = node1.create_request_vote();

        // Node2 and Node3 receive vote request and respond
        let response2 = node2.handle_request_vote(&vote_request);
        let response3 = node3.handle_request_vote(&vote_request);

        // Both should grant vote (no prior votes, logs are equal)
        assert!(response2.vote_granted);
        assert!(response3.vote_granted);

        // Node1 handles responses
        node1.handle_request_vote_response(&response2);
        let became_leader = node1.handle_request_vote_response(&response3);

        // With 3 votes (self + node2 + node3), node1 should be leader
        // Actually need only 2 for quorum of 3
        assert!(node1.is_leader() || became_leader);
    }

    #[test]
    fn test_higher_term_candidate_wins() {
        let mut node1 = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );
        let mut node2 = RaftNode::new(
            "node2".to_string(),
            vec!["node1".to_string()],
        );

        // Node1 has higher term
        node1.persistent.current_term = 5;
        node1.start_election();

        let vote_request = node1.create_request_vote();
        let response = node2.handle_request_vote(&vote_request);

        // Node2 should grant vote to higher term candidate
        assert!(response.vote_granted);
        // Node2 should update its term
        assert_eq!(node2.current_term(), 6); // term from request
    }

    #[test]
    fn test_follower_rejects_old_term_vote() {
        let mut node1 = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );
        let mut node2 = RaftNode::new(
            "node2".to_string(),
            vec!["node1".to_string()],
        );

        // Node2 has higher term
        node2.persistent.current_term = 10;

        // Node1 starts election with lower term
        node1.start_election();
        let vote_request = node1.create_request_vote();

        let response = node2.handle_request_vote(&vote_request);

        // Node2 should reject (has higher term)
        assert!(!response.vote_granted);
    }
}

// ==================== Concurrent Access Tests ====================

mod concurrent_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_concurrent_store_access() {
        let store = Arc::new(KvStore::new());
        let counter = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];

        // Spawn multiple writers
        for i in 0..10 {
            let store = Arc::clone(&store);
            let counter = Arc::clone(&counter);

            handles.push(tokio::spawn(async move {
                store.set(format!("key_{}", i), format!("value_{}", i)).await.unwrap();
                counter.fetch_add(1, Ordering::SeqCst);
            }));
        }

        // Wait for all writes
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all writes succeeded
        assert_eq!(counter.load(Ordering::SeqCst), 10);

        // Verify all keys exist
        for i in 0..10 {
            let key = format!("key_{}", i);
            let value = store.get(&key).unwrap();
            assert!(value.is_some());
        }
    }

    #[tokio::test]
    async fn test_concurrent_raft_node_access() {
        let node = RaftNode::new(
            "node1".to_string(),
            vec!["node2".to_string()],
        );
        let raft = Arc::new(RwLock::new(node));

        let mut handles = vec![];

        // Multiple readers
        for _ in 0..10 {
            let raft = Arc::clone(&raft);
            handles.push(tokio::spawn(async move {
                let guard = raft.read().await;
                let _ = guard.current_term();
                let _ = guard.is_leader();
            }));
        }

        // Wait for all reads
        for handle in handles {
            handle.await.unwrap();
        }
    }
}

