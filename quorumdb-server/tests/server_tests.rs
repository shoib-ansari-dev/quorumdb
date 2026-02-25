use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use quorumdb_core::KvStore;

// Helper to start the server on a free port
async fn start_test_server() -> (String, tokio::task::JoinHandle<()>) {
    let engine = Arc::new(KvStore::new());

    // Get a free port by binding and immediately getting the address
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind to get free port");

    let actual_addr = listener.local_addr().expect("Failed to get local address");
    let addr_str = actual_addr.to_string();

    let handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((socket, _)) => {
                    let engine_clone = Arc::clone(&engine);
                    tokio::spawn(async move {
                        quorumdb_server::handler::handle_client(socket, engine_clone).await;
                    });
                }
                Err(_) => break, // Server shutting down
            }
        }
    });

    // Give server a moment to start
    tokio::time::sleep(Duration::from_millis(50)).await;

    (addr_str, handle)
}

// Helper to connect to server and send command
async fn send_command(addr: &str, command: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    stream.write_all(format!("{}\n", command).as_bytes()).await?;
    stream.flush().await?;

    let (reader, _) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    Ok(response.trim().to_string())
}

#[tokio::test]
async fn test_server_accepts_connections() {
    let (addr, _handle) = start_test_server().await;

    // Should be able to connect
    let result = TcpStream::connect(&addr).await;
    assert!(result.is_ok(), "Failed to connect to server");
}

#[tokio::test]
async fn test_set_command() {
    let (addr, _handle) = start_test_server().await;

    let response = send_command(&addr, "SET mykey myvalue")
        .await
        .expect("Failed to send SET command");

    assert_eq!(response, "OK", "SET command should return OK");
}

#[tokio::test]
async fn test_get_command_after_set() {
    let (addr, _handle) = start_test_server().await;

    // Set a value
    let _set_response = send_command(&addr, "SET testkey testvalue")
        .await
        .expect("Failed to send SET command");

    // Get the value
    let get_response = send_command(&addr, "GET testkey")
        .await
        .expect("Failed to send GET command");

    assert_eq!(get_response, "testvalue", "GET should return the set value");
}

#[tokio::test]
async fn test_get_non_existing_key() {
    let (addr, _handle) = start_test_server().await;

    let response = send_command(&addr, "GET nonexistent")
        .await
        .expect("Failed to send GET command");

    assert_eq!(response, "NOT_FOUND", "GET on non-existent key should return NOT_FOUND");
}

#[tokio::test]
async fn test_delete_command() {
    let (addr, _handle) = start_test_server().await;

    // Set a value
    send_command(&addr, "SET deletekey deletevalue")
        .await
        .expect("Failed to send SET command");

    // Delete it
    let delete_response = send_command(&addr, "DELETE deletekey")
        .await
        .expect("Failed to send DELETE command");

    assert_eq!(delete_response, "OK", "DELETE should return OK");

    // Verify it's deleted
    let get_response = send_command(&addr, "GET deletekey")
        .await
        .expect("Failed to send GET command");

    assert_eq!(get_response, "NOT_FOUND", "Key should not exist after DELETE");
}

#[tokio::test]
async fn test_delete_non_existing_key() {
    let (addr, _handle) = start_test_server().await;

    let response = send_command(&addr, "DELETE nonexistent")
        .await
        .expect("Failed to send DELETE command");

    assert_eq!(response, "NOT_FOUND", "DELETE on non-existent key should return NOT_FOUND");
}

#[tokio::test]
async fn test_invalid_command() {
    let (addr, _handle) = start_test_server().await;

    let response = send_command(&addr, "INVALID command")
        .await
        .expect("Failed to send command");

    assert!(response.contains("ERROR"), "Invalid command should return ERROR");
}

#[tokio::test]
async fn test_multiple_sequential_operations() {
    let (addr, _handle) = start_test_server().await;

    // SET key1
    let resp1 = send_command(&addr, "SET key1 value1")
        .await
        .expect("Failed to SET key1");
    assert_eq!(resp1, "OK");

    // SET key2
    let resp2 = send_command(&addr, "SET key2 value2")
        .await
        .expect("Failed to SET key2");
    assert_eq!(resp2, "OK");

    // GET key1
    let resp3 = send_command(&addr, "GET key1")
        .await
        .expect("Failed to GET key1");
    assert_eq!(resp3, "value1");

    // GET key2
    let resp4 = send_command(&addr, "GET key2")
        .await
        .expect("Failed to GET key2");
    assert_eq!(resp4, "value2");

    // DELETE key1
    let resp5 = send_command(&addr, "DELETE key1")
        .await
        .expect("Failed to DELETE key1");
    assert_eq!(resp5, "OK");

    // Verify key1 deleted
    let resp6 = send_command(&addr, "GET key1")
        .await
        .expect("Failed to GET key1 after delete");
    assert_eq!(resp6, "NOT_FOUND");

    // Verify key2 still exists
    let resp7 = send_command(&addr, "GET key2")
        .await
        .expect("Failed to GET key2");
    assert_eq!(resp7, "value2");
}

#[tokio::test]
async fn test_concurrent_clients() {
    let (addr, _handle) = start_test_server().await;
    let mut handles = vec![];

    // Spawn 10 concurrent clients
    for i in 0..10 {
        let addr_clone = addr.clone();
        let handle = tokio::spawn(async move {
            let key = format!("key{}", i);
            let value = format!("value{}", i);

            // SET
            let set_resp = send_command(&addr_clone, &format!("SET {} {}", key, value))
                .await
                .expect("Failed to SET");
            assert_eq!(set_resp, "OK");

            // GET
            let get_resp = send_command(&addr_clone, &format!("GET {}", key))
                .await
                .expect("Failed to GET");
            assert_eq!(get_resp, value);

            // DELETE
            let del_resp = send_command(&addr_clone, &format!("DELETE {}", key))
                .await
                .expect("Failed to DELETE");
            assert_eq!(del_resp, "OK");
        });
        handles.push(handle);
    }

    // Wait for all clients to complete
    for handle in handles {
        handle.await.expect("Client task panicked");
    }
}

#[tokio::test]
async fn test_overwrite_existing_key() {
    let (addr, _handle) = start_test_server().await;

    // SET initial value
    send_command(&addr, "SET overwrite_key initial_value")
        .await
        .expect("Failed to SET");

    // Verify initial value
    let resp1 = send_command(&addr, "GET overwrite_key")
        .await
        .expect("Failed to GET");
    assert_eq!(resp1, "initial_value");

    // Overwrite with new value
    send_command(&addr, "SET overwrite_key new_value")
        .await
        .expect("Failed to SET");

    // Verify new value
    let resp2 = send_command(&addr, "GET overwrite_key")
        .await
        .expect("Failed to GET");
    assert_eq!(resp2, "new_value");
}

#[tokio::test]
async fn test_empty_command() {
    let (addr, _handle) = start_test_server().await;

    let response = send_command(&addr, "")
        .await
        .expect("Failed to send empty command");

    assert!(response.contains("ERROR"), "Empty command should return ERROR");
}

#[tokio::test]
async fn test_set_with_missing_value() {
    let (addr, _handle) = start_test_server().await;

    let response = send_command(&addr, "SET onlykey")
        .await
        .expect("Failed to send incomplete SET");

    assert!(response.contains("ERROR"), "SET without value should return ERROR");
}

#[tokio::test]
async fn test_get_with_extra_args() {
    let (addr, _handle) = start_test_server().await;

    let response = send_command(&addr, "GET key extra")
        .await
        .expect("Failed to send GET with extra args");

    assert!(response.contains("ERROR"), "GET with extra args should return ERROR");
}

#[tokio::test]
async fn test_case_insensitive_commands() {
    let (addr, _handle) = start_test_server().await;

    // SET with lowercase
    let resp1 = send_command(&addr, "set lowercase_key lowercase_value")
        .await
        .expect("Failed");
    assert_eq!(resp1, "OK");

    // GET with uppercase
    let resp2 = send_command(&addr, "GET lowercase_key")
        .await
        .expect("Failed");
    assert_eq!(resp2, "lowercase_value");

    // DELETE with mixed case
    let resp3 = send_command(&addr, "DeLeTe lowercase_key")
        .await
        .expect("Failed");
    assert_eq!(resp3, "OK");
}

#[tokio::test]
async fn test_whitespace_handling() {
    let (addr, _handle) = start_test_server().await;

    // Command with extra spaces
    let response = send_command(&addr, "SET   key_with_spaces   value_here")
        .await
        .expect("Failed");
    assert_eq!(response, "OK");

    // Verify it was set correctly
    let get_resp = send_command(&addr, "GET key_with_spaces")
        .await
        .expect("Failed");
    assert_eq!(get_resp, "value_here");
}

#[tokio::test]
async fn test_persistence_across_clients() {
    let (addr, _handle) = start_test_server().await;

    // Client 1 sets a value
    send_command(&addr, "SET persistent_key persistent_value")
        .await
        .expect("Failed");

    // Client 2 reads the value
    let response = send_command(&addr, "GET persistent_key")
        .await
        .expect("Failed");
    assert_eq!(response, "persistent_value");

    // Client 3 modifies it
    send_command(&addr, "SET persistent_key modified_value")
        .await
        .expect("Failed");

    // Client 4 verifies the modification
    let response2 = send_command(&addr, "GET persistent_key")
        .await
        .expect("Failed");
    assert_eq!(response2, "modified_value");
}

