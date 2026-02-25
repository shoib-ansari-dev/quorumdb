use std::sync::Arc;
use quorumdb_core::KvStore;

mod server;
mod protocol;
mod handler;

#[tokio::main]
async fn main() {
    let engine = Arc::new(KvStore::new());

    if let Err(e)= engine.init_wal("quorum.wal").await {
        eprintln!("Failed to initialize WAL: {}", e);
        std::process::exit(1);
    }
    println!("Server running on 127.0.0.1:7000");

    server::run("127.0.0.1:7000", engine).await;
}
