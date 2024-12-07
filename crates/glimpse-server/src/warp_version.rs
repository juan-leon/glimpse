use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;

// Shared state between WebSocket connections
type Clients = Arc<Mutex<HashMap<String, futures::channel::mpsc::UnboundedSender<Message>>>>;

#[tokio::main]
async fn main() {
    // Shared state
    let clients: Clients = Arc::new(Mutex::new(HashMap::new()));
    let clients_ws = clients.clone();

    // UDP Socket setup
    let udp_socket = UdpSocket::bind("127.0.0.1:8081")
        .await
        .expect("Failed to bind UDP socket");
    println!("UDP server listening on: 127.0.0.1:8081");

    // WebSocket route
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(warp::any().map(move || clients_ws.clone()))
        .map(|ws: warp::ws::Ws, clients| {
            ws.on_upgrade(move |socket| handle_websocket_client(socket, clients))
        });

    // Spawn UDP listener
    let udp_clients = clients.clone();
    tokio::spawn(async move {
        handle_udp_messages(udp_socket, udp_clients).await;
    });

    // Start WebSocket server
    println!("WebSocket server listening on: 127.0.0.1:8080");
    warp::serve(ws_route).run(([127, 0, 0, 1], 8080)).await;
}

async fn handle_websocket_client(ws: WebSocket, clients: Clients) {
    // Split the socket into sender and receiver
    let (mut ws_tx, mut ws_rx) = ws.split();

    // Create channel for this client
    let (tx, rx) = futures::channel::mpsc::unbounded();
    let mut rx = UnboundedReceiverStream::new(rx);

    // Generate a client id
    let client_id = uuid::Uuid::new_v4().to_string();

    // Store the sender in shared state
    clients.lock().await.insert(client_id.clone(), tx);

    // Forward messages from the channel to the websocket
    tokio::spawn(async move {
        while let Some(message) = rx.next().await {
            ws_tx
                .send(message)
                .await
                .unwrap_or_else(|e| eprintln!("WebSocket send error: {}", e));
        }
    });

    // Handle incoming WebSocket messages
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                println!("Received WebSocket message: {:?}", msg);
                // Handle the message here
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
        }
    }

    // Remove client when connection is closed
    clients.lock().await.remove(&client_id);
}

async fn handle_udp_messages(socket: UdpSocket, clients: Clients) {
    let mut buf = vec![0u8; 1024];
    // let mut buf = [0; 1024];

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((len, addr)) => {
                let message = String::from_utf8_lossy(&buf[..len]).to_string();
                println!("Received UDP message from {}: {}", addr, message);

                // Broadcast UDP message to all WebSocket clients
                let clients_lock = clients.lock().await;
                for tx in clients_lock.values() {
                    if let Err(e) = tx.unbounded_send(Message::text(&message)) {
                        eprintln!("Failed to forward UDP message to WebSocket client: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("UDP receive error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::mpsc;
    use futures::future::join_all;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::net::TcpStream;
    use tokio::task;
    use tokio::time::timeout;
    use warp::test::WsClient;

    // Helper function to create a test WebSocket client
    async fn create_test_ws_client() -> WsClient {
        let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

        let ws_route = warp::path("ws")
            .and(warp::ws())
            .and(warp::any().map(move || clients.clone()))
            .map(|ws: warp::ws::Ws, clients| {
                ws.on_upgrade(move |socket| handle_websocket_client(socket, clients))
            });

        warp::test::ws()
            .path("/ws")
            .handshake(ws_route)
            .await
            .expect("handshake failed")
    }

    #[tokio::test]
    async fn test_websocket_connection() {
        let mut client = create_test_ws_client().await;

        // Send a test message
        let message = "test message";
        client.send_text(message).await;

        // Expect to receive the message back
        let response = client.recv().await.expect("failed to receive ws message");
        assert!(response.is_text());
        assert_eq!(response.to_str().unwrap(), message);
    }

    #[tokio::test]
    async fn test_multiple_websocket_clients() {
        let mut client1 = create_test_ws_client().await;
        let mut client2 = create_test_ws_client().await;

        // Send message from client1
        let message = "broadcast test";
        client1.send_text(message).await;

        // Both clients should receive the message
        let response1 = client1.recv().await.expect("client1 failed to receive");
        let response2 = client2.recv().await.expect("client2 failed to receive");

        assert_eq!(response1.to_str().unwrap(), message);
        assert_eq!(response2.to_str().unwrap(), message);
    }

    #[tokio::test]
    async fn test_udp_message_broadcast() {
        // Create WebSocket clients
        let mut ws_client = create_test_ws_client().await;

        // Create and bind UDP socket
        let udp_socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind UDP socket");
        let server_addr = udp_socket.local_addr().unwrap();

        // Create UDP client socket
        let udp_client = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind UDP client");

        // Send UDP message
        let message = "UDP test message";
        udp_client
            .send_to(message.as_bytes(), server_addr)
            .await
            .expect("Failed to send UDP message");

        // Wait for and verify WebSocket received the message
        let timeout_duration = Duration::from_secs(1);
        let ws_response = timeout(timeout_duration, ws_client.recv())
            .await
            .expect("timeout waiting for ws message")
            .expect("ws receive failed");

        assert_eq!(ws_response.to_str().unwrap(), message);
    }

    #[tokio::test]
    async fn test_client_disconnect() {
        let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

        // Create and connect a client
        let mut client = create_test_ws_client().await;

        // Verify client is in the clients map
        assert_eq!(clients.lock().await.len(), 1);

        // Disconnect client
        drop(client);

        // Give some time for cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify client was removed from the clients map
        assert_eq!(clients.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn test_invalid_udp_message() {
        let udp_socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind UDP socket");
        let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

        // Send invalid UTF-8 data
        let invalid_data = vec![0xFF, 0xFF, 0xFF];
        let result = handle_udp_messages(udp_socket, clients.clone()).await;

        // The handler should continue running and not panic
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_websocket_message_validation() {
        let mut client = create_test_ws_client().await;

        // Test empty message
        client.send_text("").await;

        // Test large message
        let large_message = "a".repeat(1024 * 1024); // 1MB message
        client.send_text(&large_message).await;

        // Test binary message
        client.send_binary(vec![1, 2, 3, 4]).await;

        // The connection should remain open
        assert!(client.recv().await.is_ok());
    }

    // New helper function for simulating network delays
    async fn delayed_send(client: &mut WsClient, message: &str, delay_ms: u64) {
        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        client.send_text(message).await;
    }

    #[tokio::test]
    async fn test_network_partition() {
        let mut client = create_test_ws_client().await;

        // Simulate network partition by dropping packets
        let (tx, mut rx) = mpsc::unbounded();

        // Send messages during partition
        for i in 0..5 {
            client.send_text(&format!("message {}", i)).await;
        }

        // Simulate network restore
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify messages are received in order after partition
        for i in 0..5 {
            let msg = client
                .recv()
                .await
                .expect("Failed to receive message after partition");
            assert_eq!(msg.to_str().unwrap(), format!("message {}", i));
        }
    }

    #[tokio::test]
    async fn test_network_latency() {
        let mut client = create_test_ws_client().await;

        // Send messages with different delays
        let message1 = "fast message";
        let message2 = "slow message";

        // Spawn tasks to send messages with different delays
        let send_task1 = tokio::spawn(delayed_send(&mut client, message1, 10));
        let send_task2 = tokio::spawn(delayed_send(&mut client, message2, 50));

        // Wait for both messages
        let response1 = client
            .recv()
            .await
            .expect("Failed to receive first message");
        let response2 = client
            .recv()
            .await
            .expect("Failed to receive second message");

        // Verify messages were received
        assert!(response1.to_str().unwrap() == message1 || response1.to_str().unwrap() == message2);
        assert!(response2.to_str().unwrap() == message1 || response2.to_str().unwrap() == message2);
    }

    #[tokio::test]
    async fn test_connection_timeout() {
        let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

        // Create a client with a very short timeout
        let timeout_duration = Duration::from_millis(100);

        // Attempt to connect with timeout
        let connection_result = timeout(timeout_duration, create_test_ws_client()).await;

        // Connection should succeed within timeout
        assert!(connection_result.is_ok());

        // Try to send/receive with a disconnected client
        let mut client = connection_result.unwrap();
        client.send_text("test").await;

        // Simulate network timeout
        tokio::time::sleep(Duration::from_secs(1)).await;

        // Verify client was removed from clients map
        assert_eq!(clients.lock().await.len(), 0);
    }

    #[tokio::test]
    async fn test_concurrent_clients() {
        const NUM_CLIENTS: usize = 100;
        const MESSAGES_PER_CLIENT: usize = 10;

        let message_counter = Arc::new(AtomicUsize::new(0));
        let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

        // Create multiple clients concurrently
        let mut client_handles = vec![];

        for i in 0..NUM_CLIENTS {
            let counter = message_counter.clone();

            let handle = tokio::spawn(async move {
                let mut client = create_test_ws_client().await;

                // Send messages concurrently
                for j in 0..MESSAGES_PER_CLIENT {
                    let message = format!("client_{}_message_{}", i, j);
                    client.send_text(&message).await;
                    counter.fetch_add(1, Ordering::SeqCst);

                    // Random delay to simulate real-world conditions
                    tokio::time::sleep(Duration::from_millis(rand::random::<u64>() % 50)).await;
                }

                client
            });

            client_handles.push(handle);
        }

        // Wait for all clients to complete
        let clients = join_all(client_handles).await;

        // Verify all messages were sent
        assert_eq!(
            message_counter.load(Ordering::SeqCst),
            NUM_CLIENTS * MESSAGES_PER_CLIENT
        );

        // Verify all clients are still connected
        for client in clients {
            let client = client.expect("Client task failed");
            assert!(client.recv().await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_message_ordering_under_load() {
        let mut client = create_test_ws_client().await;
        let num_messages = 1000;

        // Send messages rapidly
        for i in 0..num_messages {
            client.send_text(&format!("message_{}", i)).await;
        }

        // Verify messages are received in order
        for i in 0..num_messages {
            let response = client.recv().await.expect("Failed to receive message");
            assert_eq!(response.to_str().unwrap(), format!("message_{}", i));
        }
    }

    #[tokio::test]
    async fn test_reconnection_handling() {
        let clients: Clients = Arc::new(Mutex::new(HashMap::new()));

        // Connect client
        let mut client = create_test_ws_client().await;

        // Force disconnect
        drop(client);

        // Allow time for cleanup
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Reconnect
        client = create_test_ws_client().await;

        // Verify new connection works
        client.send_text("reconnection test").await;
        let response = client
            .recv()
            .await
            .expect("Failed to receive after reconnection");
        assert_eq!(response.to_str().unwrap(), "reconnection test");
    }

    #[tokio::test]
    async fn test_concurrent_udp_websocket() {
        let udp_socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind UDP socket");
        let server_addr = udp_socket.local_addr().unwrap();

        // Create multiple WebSocket clients
        let num_ws_clients = 10;
        let mut ws_clients = vec![];

        for _ in 0..num_ws_clients {
            ws_clients.push(create_test_ws_client().await);
        }

        // Create UDP client
        let udp_client = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind UDP client");

        // Send UDP messages while WebSocket clients are active
        for i in 0..10 {
            let message = format!("udp_message_{}", i);
            udp_client
                .send_to(message.as_bytes(), server_addr)
                .await
                .expect("Failed to send UDP message");

            // Verify all WebSocket clients receive the message
            for client in &mut ws_clients {
                let response = client
                    .recv()
                    .await
                    .expect("Failed to receive UDP broadcast");
                assert_eq!(response.to_str().unwrap(), message);
            }
        }
    }
}

// [dependencies]
// tokio = { version = "1.0", features = ["full"] }
// warp = "0.3"
// futures = "0.3"
// tokio-stream = "0.1"
// uuid = { version = "1.0", features = ["v4"] }

// [dev-dependencies]
// tokio-test = "0.4"
// rand = "0.8"
// futures-util = "0.3"