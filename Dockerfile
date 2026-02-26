# Build stage
FROM rust:1.85 AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source
COPY src ./src
COPY quorumdb-core ./quorumdb-core
COPY quorumdb-server ./quorumdb-server
COPY quorumdb-proto ./quorumdb-proto
COPY quorumdb-client ./quorumdb-client
COPY quorumdb-bench ./quorumdb-bench

# Build release binary
RUN cargo build --release -p quorumdb-server

# Runtime stage - use minimal distroless image (~20MB)
FROM gcr.io/distroless/cc-debian12

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/quorumdb-server /usr/local/bin/quorumdb-server

# Expose ports
EXPOSE 7000 7001

# Set environment defaults
ENV NODE_ID=node1 \
    BIND_ADDR=0.0.0.0 \
    CLIENT_PORT=7000 \
    RAFT_PORT=7001 \
    RUST_LOG=info

# Run the server
CMD ["quorumdb-server"]

