use serde::{Deserialize, Serialize};

/// Cluster configuration - can be loaded from file or env
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    pub node_id: String,
    pub bind_addr: String,
    pub peers: Vec<(String, String)>,
    pub client_port: u16,
    pub raft_port: u16,
}

impl ClusterConfig {
    /// Load from environment variables
    pub fn from_env() -> Self {
        let node_id = std::env::var("NODE_ID").unwrap_or_else(|_| "node1".to_string());
        let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string());
        let client_port = std::env::var("CLIENT_PORT")
            .unwrap_or_else(|_| "7000".to_string())
            .parse::<u16>()
            .unwrap_or(7000);
        let raft_port = std::env::var("RAFT_PORT")
            .unwrap_or_else(|_| "7001".to_string())
            .parse::<u16>()
            .unwrap_or(7001);

        // Parse peers from PEERS env var (format: "node2:node2:7001,node3:node3:7001")
        let peers_str = std::env::var("PEERS").unwrap_or_default();
        let peers = peers_str
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|peer_str| {
                let parts: Vec<&str> = peer_str.split(':').collect();
                if parts.len() >= 2 {
                    (parts[0].to_string(), format!("{}:{}", parts[1], parts[2]))
                } else {
                    (String::new(), String::new())
                }
            })
            .filter(|(id, _)| !id.is_empty())
            .collect();

        Self {
            node_id,
            bind_addr,
            peers,
            client_port,
            raft_port,
        }
    }

    /// Create default 3-node cluster for testing
    pub fn default_cluster() -> Self {
        Self {
            node_id: "node1".to_string(),
            bind_addr: "0.0.0.0".to_string(),
            peers: vec![
                ("node2".to_string(), "node2:7001".to_string()),
                ("node3".to_string(), "node3:7001".to_string()),
            ],
            client_port: 7000,
            raft_port: 7001,
        }
    }

    /// Get peer IDs
    pub fn peer_ids(&self) -> Vec<String> {
        self.peers.iter().map(|(id, _)| id.clone()).collect()
    }

    /// Get peer addresses
    pub fn peer_addresses(&self) -> Vec<String> {
        self.peers.iter().map(|(_, addr)| addr.clone()).collect()
    }

    /// Get client listen address
    pub fn client_addr(&self) -> String {
        format!("{}:{}", self.bind_addr, self.client_port)
    }

    /// Get raft peer listen address
    pub fn raft_addr(&self) -> String {
        format!("{}:{}", self.bind_addr, self.raft_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_config_default() {
        let config = ClusterConfig::default_cluster();
        assert_eq!(config.node_id, "node1");
        assert_eq!(config.peers.len(), 2);
        assert_eq!(config.client_port, 7000);
        assert_eq!(config.raft_port, 7001);
    }

    #[test]
    fn test_peer_ids() {
        let config = ClusterConfig::default_cluster();
        let ids = config.peer_ids();
        assert_eq!(ids, vec!["node2", "node3"]);
    }
}

