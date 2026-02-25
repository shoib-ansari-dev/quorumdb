use std::sync::Arc;
use tokio::net::TcpListener;

use quorumdb_core::KvStore;

use crate::handler::handle_client;

pub async fn run(addr: &str, engine: Arc<KvStore>) {

    let listener = TcpListener::bind(addr)
        .await
        .expect("Failed to bind");

    loop{
        let (socket, _ )= listener
            .accept()
            .await
            .expect("Failed to accept connection");
        let engine= Arc::clone(&engine);

        tokio::spawn(async move {
            handle_client(socket, engine).await;
        });
    }
}