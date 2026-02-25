use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use quorumdb_core::KvStore;

use crate::protocol::{parse, Command};

pub async fn handle_client(socket: TcpStream, engine: Arc<KvStore>) {
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();

        let bytes_read = match reader.read_line(&mut line).await {
            Ok(0) => return, // client closed connection
            Ok(n) => n,
            Err(_) => return,
        };

        if bytes_read == 0 {
            return;
        }

        let response = match parse(&line) {
            Ok(cmd) => execute(cmd, &engine),
            Err(err) => format!("ERROR: {}\n", err),
        };

        if writer.write_all(response.as_bytes()).await.is_err() {
            return;
        }
    }
}

fn execute(cmd: Command, engine: &KvStore) -> String {
    match cmd {
        Command::Set { key, value } => {
            engine.set(key, value);
            "OK\n".to_string()
        }
        Command::Get { key } => {
            match engine.get(&key) {
                Ok(Some(val)) => format!("{}\n", val),
                Ok(None) => "NOT_FOUND\n".to_string(),
                Err(e) => format!("ERROR: {}\n", e),
            }
        }
        Command::Delete { key } => {
            match engine.delete(&key) {
                Ok(Some(_)) => "OK\n".to_string(),
                Ok(None) => "NOT_FOUND\n".to_string(),
                Err(e) => format!("ERROR: {}\n", e),
            }
        }
    }
}
