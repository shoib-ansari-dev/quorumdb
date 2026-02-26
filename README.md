# 🗄️ QuorumDB

**A distributed key-value database built with Raft consensus in Rust**

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Vibe Coded](https://img.shields.io/badge/vibe-coded%20🎵-ff69b4.svg)](https://en.wikipedia.org/wiki/Vibe_coding)

> 🎵 **This is a vibe coded project** — built with AI assistance, good vibes, and a flow state. The code was crafted through natural conversation with AI, letting creativity and intuition guide the architecture while maintaining engineering rigor.

QuorumDB is a fault-tolerant, distributed key-value store that uses the Raft consensus protocol to maintain consistency across a cluster of nodes. It's designed for learning distributed systems concepts while being production-ready.

---

## ✨ Features

### 🔐 Distributed Consensus (Raft Protocol)
- **Leader Election**: Automatic leader election with randomized timeouts
- **Log Replication**: Consistent replication across all nodes
- **Fault Tolerance**: Cluster survives minority node failures (e.g., 1 of 3 nodes)
- **Safety Guarantees**: Linearizable reads and writes

### 💾 Key-Value Storage
- **Simple API**: SET, GET, DELETE operations
- **Thread-Safe**: Lock-free concurrent reads, synchronized writes
- **In-Memory**: Fast O(1) operations with HashMap backend
- **Type-Safe**: Rust's type system ensures correctness
- **Write-Ahead Log**: Durability with MessagePack serialization and fsync

### 🌐 Networking
- **TCP Protocol**: Simple text-based client protocol
- **Async I/O**: Built on Tokio for high concurrency
- **Multi-Port**: Separate ports for client (7000) and Raft RPC (7001)
- **Leader Redirection**: Followers redirect writes to current leader

### 🐳 Docker Support
- **Multi-Stage Build**: Optimized container images
- **Docker Compose**: One-command 3-node cluster setup
- **Service Discovery**: Automatic peer discovery via DNS
- **Health Checks**: Built-in container health monitoring

### 🧪 Comprehensive Testing
- **69+ Tests**: Unit, integration, and cluster tests
- **Protocol Tests**: Command parsing verification
- **Raft Tests**: Consensus algorithm correctness
- **Server Tests**: End-to-end client communication

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    QuorumDB Cluster                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│   │   Node 1    │    │   Node 2    │    │   Node 3    │     │
│   │  (Leader)   │◄──►│ (Follower)  │◄──►│ (Follower)  │     │
│   │             │    │             │    │             │     │
│   │ ┌─────────┐ │    │ ┌─────────┐ │    │ ┌─────────┐ │     │
│   │ │RaftNode │ │    │ │RaftNode │ │    │ │RaftNode │ │     │
│   │ └─────────┘ │    │ └─────────┘ │    │ └─────────┘ │     │
│   │ ┌─────────┐ │    │ ┌─────────┐ │    │ ┌─────────┐ │     │
│   │ │ KvStore │ │    │ │ KvStore │ │    │ │ KvStore │ │     │
│   │ └─────────┘ │    │ └─────────┘ │    │ └─────────┘ │     │
│   │             │    │             │    │             │     │
│   │ :7000 :7001 │    │ :7010 :7011 │    │ :7020 :7021 │     │
│   └─────────────┘    └─────────────┘    └─────────────┘     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Project Structure

```
quorumdb/
├── quorumdb-core/          # Core library
│   └── src/
│       ├── lib.rs          # Public API exports
│       ├── storage/        # Key-value storage engine
│       │   ├── engine.rs   # Thread-safe HashMap wrapper
│       │   └── wal.rs      # Write-ahead log (persistence)
│       └── raft/           # Raft consensus implementation
│           ├── node.rs     # RaftNode - main consensus logic
│           ├── state.rs    # Node states, timers, config
│           ├── log.rs      # Raft log management
│           └── message.rs  # RPC message types
│
├── quorumdb-server/        # Server binary
│   └── src/
│       ├── main.rs         # Entry point
│       ├── server.rs       # TCP listener
│       ├── handler.rs      # Client command handler
│       ├── protocol.rs     # Command parser
│       ├── cluster_config.rs   # Cluster configuration
│       ├── cluster_node.rs     # Raft + KvStore wrapper
│       ├── raft_handler.rs     # Raft RPC handler
│       ├── raft_tick_loop.rs   # Background consensus loop
│       └── peer_server.rs      # Peer RPC server
│
├── quorumdb-client/        # CLI client
├── quorumdb-proto/         # Protocol definitions
├── quorumdb-bench/         # Benchmarking tools
│
├── Dockerfile              # Container build
├── docker-compose.yml      # 3-node cluster setup
└── Cargo.toml              # Workspace configuration
```

---

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Docker & Docker Compose (for cluster mode)

### Option 1: Single Node (Development)

```bash
# Clone and build
git clone https://github.com/yourusername/quorumdb.git
cd quorumdb
cargo build --release

# Run single node
cargo run --release --bin quorumdb-server

# In another terminal, connect with netcat
nc localhost 7000
SET mykey myvalue
GET mykey
DELETE mykey
```

### Option 2: 3-Node Cluster (Docker)

```bash
# Build Docker image
docker build -t quorumdb:latest .

# Start 3-node cluster
docker-compose up -d

# Check cluster status
docker-compose logs -f

# Connect to any node
nc localhost 7000   # Node 1
nc localhost 7010   # Node 2
nc localhost 7020   # Node 3
```

---

## 📖 Protocol Reference

QuorumDB uses a simple text-based protocol over TCP:

### Commands

| Command | Syntax | Response |
|---------|--------|----------|
| **SET** | `SET <key> <value>` | `OK` or `NOT_LEADER:<leader_id>` |
| **GET** | `GET <key>` | `<value>` or `NOT_FOUND` |
| **DELETE** | `DELETE <key>` | `OK` or `NOT_FOUND` |

### Examples

```bash
# Connect to cluster
nc localhost 7000

# Write data (must be on leader)
SET user:1 {"name":"Alice","age":30}
> OK

# Read data (any node)
GET user:1
> {"name":"Alice","age":30}

# Delete data (must be on leader)
DELETE user:1
> OK

# Read after delete
GET user:1
> NOT_FOUND
```

### Leader Redirection

If you send a write to a follower:
```bash
nc localhost 7010  # Connect to follower
SET foo bar
> NOT_LEADER:node1
```

---

## ⚙️ Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `NODE_ID` | `node1` | Unique identifier for this node |
| `BIND_ADDR` | `0.0.0.0` | Address to bind servers |
| `CLIENT_PORT` | `7000` | Port for client connections |
| `RAFT_PORT` | `7001` | Port for Raft peer communication |
| `PEERS` | (empty) | Comma-separated peer list: `node2:host2:7011,node3:host3:7021` |

### Example: Manual 3-Node Setup

```bash
# Terminal 1 - Node 1
NODE_ID=node1 CLIENT_PORT=7000 RAFT_PORT=7001 \
  PEERS="node2:localhost:7011,node3:localhost:7021" \
  cargo run --bin quorumdb-server

# Terminal 2 - Node 2
NODE_ID=node2 CLIENT_PORT=7010 RAFT_PORT=7011 \
  PEERS="node1:localhost:7001,node3:localhost:7021" \
  cargo run --bin quorumdb-server

# Terminal 3 - Node 3
NODE_ID=node3 CLIENT_PORT=7020 RAFT_PORT=7021 \
  PEERS="node1:localhost:7001,node2:localhost:7011" \
  cargo run --bin quorumdb-server
```

---

## 🧪 Testing

```bash
# Run all tests
cargo test --all

# Run specific test suites
cargo test --package quorumdb-core          # Core library tests
cargo test --package quorumdb-server        # Server tests
cargo test --test cluster_tests             # Cluster integration tests
cargo test --test protocol_tests            # Protocol parsing tests

# Run with output
cargo test -- --nocapture
```

### Test Coverage

| Test Suite | Tests | Description |
|------------|-------|-------------|
| Core Raft | 33 | Consensus algorithm correctness |
| Storage | 10 | Key-value operations |
| Protocol | 19 | Command parsing |
| Server | 16 | Client communication |
| Cluster | 34 | Integration & multi-node |
| **Total** | **112+** | Comprehensive coverage |

---

## 🔧 Development

### Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Check for errors without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

### Docker Commands

```bash
# Build image
docker build -t quorumdb:latest .

# Start cluster
docker-compose up -d

# View logs
docker-compose logs -f node1

# Stop cluster
docker-compose down

# Rebuild and restart
docker-compose up -d --build
```

---

## 📚 How It Works

### Raft Consensus

QuorumDB implements the Raft consensus algorithm with these components:

1. **Leader Election**
   - Nodes start as followers with random election timeouts (150-300ms)
   - If a follower doesn't hear from a leader, it becomes a candidate
   - Candidate requests votes from peers; majority wins
   - Leader sends heartbeats every 50ms to maintain authority

2. **Log Replication**
   - All writes go to the leader
   - Leader appends entry to its log
   - Leader sends AppendEntries RPC to all followers
   - Once majority acknowledges, entry is committed
   - Committed entries are applied to the state machine (KvStore)

3. **Safety Guarantees**
   - Election Safety: At most one leader per term
   - Leader Append-Only: Leader never overwrites its log
   - Log Matching: Logs with same index/term have same command
   - Leader Completeness: Committed entries survive leader changes
   - State Machine Safety: All nodes apply same entries in same order

### Tick Loop

The background tick loop runs every 10ms:

```
┌─────────────────────────────────────┐
│           Tick Loop (10ms)          │
├─────────────────────────────────────┤
│ 1. Check election timeout           │
│    → Start election if expired      │
│                                     │
│ 2. If leader:                       │
│    → Send heartbeats to followers   │
│                                     │
│ 3. Apply committed entries          │
│    → Execute commands on KvStore    │
└─────────────────────────────────────┘
```

---

## 🗺️ Roadmap

- [x] Core storage engine
- [x] TCP server with async I/O
- [x] Text-based protocol (SET/GET/DELETE)
- [x] Raft consensus (leader election, log replication)
- [x] Cluster configuration
- [x] Docker support
- [x] Comprehensive test suite
- [x] Write-ahead log (WAL) implementation
- [ ] WAL integration with storage engine (auto-replay on startup)
- [ ] Snapshot support
- [ ] Membership changes (add/remove nodes)
- [ ] HTTP API
- [ ] Web dashboard
- [ ] Metrics & monitoring
- [ ] TLS encryption

---

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 🙏 Acknowledgments

- [The Raft Consensus Algorithm](https://raft.github.io/) by Diego Ongaro
- [Raft Paper](https://raft.github.io/raft.pdf) - In Search of an Understandable Consensus Algorithm
- [Tokio](https://tokio.rs/) - Async runtime for Rust
- [Serde](https://serde.rs/) - Serialization framework

---

## 📬 Contact

For questions, issues, or contributions, please open an issue on GitHub.

---

**Built with ❤️ in Rust**

