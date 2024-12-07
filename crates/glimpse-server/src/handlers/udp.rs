use crate::types::Clients;
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::broadcast;
use tracing::{debug, error, info, instrument, warn};
use warp::ws::Message;

#[instrument(skip(socket, clients, shutdown))]
pub async fn handle_udp_messages(
    socket: UdpSocket,
    clients: Clients,
    mut shutdown: broadcast::Receiver<()>,
) {
    let mut buf = [0; 1024];

    loop {
        select! {
            result = socket.recv_from(&mut buf) => {
                match result {
                    Ok((len, addr)) => {
                        debug!(?addr, bytes = len, "Received UDP datagram");
                        info!(?addr, bytes = len, "Received UDP datagram");
                        match String::from_utf8(buf[..len].to_vec()) {
                            Ok(message) => {
                                info!(?addr, ?message, "Decoded UDP message");
                                let clients_lock = clients.lock().await;
                                for tx in clients_lock.values() {
                                    if let Err(e) = tx.send(Message::text(&message)) {
                                        error!("Failed to forward UDP message to WebSocket client: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(?addr, ?e, "Invalid UTF-8 in UDP message");
                            }
                        }
                    }
                    Err(e) => {
                        error!(?e, "UDP receive error");
                    }
                }
            }
            _ = shutdown.recv() => {
                info!("UDP handler shutting down");
                break;
            }
        }
    }
}
