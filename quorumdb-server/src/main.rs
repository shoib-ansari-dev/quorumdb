use std::sync::Arc;
use quorumdb_core::KvStore;

mod server;
mod protocol;
mod handler;

#[tokio::main]
async fn main() {
    let engine = Arc::new(KvStore::new());

    println!("Server running on 127.0.0.1:7000");

    server::run("127.0.0.1:7000", engine).await;
}
