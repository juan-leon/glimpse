use once_cell::sync::Lazy;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::broadcast;
use tracing::{debug, error, info, instrument};
use warp::Filter;

use crate::error::ServerError;
use crate::handlers::{handle_udp_messages, handle_websocket_client};
use crate::types::Clients;

static SHUTDOWN_CHANNEL: Lazy<broadcast::Sender<()>> = Lazy::new(|| broadcast::channel(1).0);

pub struct Server {
    clients: Clients,
    ws_addr: SocketAddr,
    udp_addr: SocketAddr,
}

impl Server {
    pub fn new(ws_addr: SocketAddr, udp_addr: SocketAddr) -> Self {
        info!(?ws_addr, ?udp_addr, "Creating new server instance");
        let clients = Clients::default();
        Server {
            clients,
            ws_addr,
            udp_addr,
        }
    }

    fn create_ws_route(
        &self,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let clients = self.clients.clone();
        let shutdown = SHUTDOWN_CHANNEL.clone();
        warp::ws()
            .and(warp::any().map(move || (clients.clone(), shutdown.subscribe())))
            .map(|ws: warp::ws::Ws, (clients, shutdown)| {
                ws.on_upgrade(move |socket| handle_websocket_client(socket, clients, shutdown))
            })
    }

    #[instrument(skip(self))]
    async fn create_udp_socket(&self) -> Result<UdpSocket, ServerError> {
        debug!(?self.udp_addr, "Creating UDP socket");
        match UdpSocket::bind(self.udp_addr).await {
            Ok(socket) => {
                info!("UDP socket created successfully");
                Ok(socket)
            }
            Err(e) => {
                error!(?e, "Failed to create UDP socket");
                Err(ServerError::Io(e))
            }
        }
    }

    pub async fn run(&self) -> Result<(), ServerError> {
        info!("Starting server");

        let udp_socket = self.create_udp_socket().await?;
        info!(?self.udp_addr, "UDP server listening");

        let udp_clients = self.clients.clone();
        let udp_shutdown = SHUTDOWN_CHANNEL.subscribe();
        let udp_handle = tokio::spawn(async move {
            handle_udp_messages(udp_socket, udp_clients, udp_shutdown).await
        });

        info!(?self.ws_addr, "WebSocket server starting");
        let mut ws_shutdown = SHUTDOWN_CHANNEL.subscribe();
        let (_addr, server) = warp::serve(self.create_ws_route()).bind_with_graceful_shutdown(
            self.ws_addr,
            async move {
                let _ = ws_shutdown.recv().await;
            },
        );

        let shutdown_sender = SHUTDOWN_CHANNEL.clone();
        tokio::spawn(async move {
            if let Ok(()) = tokio::signal::ctrl_c().await {
                info!("Received Ctrl+C signal");
                if let Err(e) = shutdown_sender.send(()) {
                    error!(?e, "Failed to send shutdown signal");
                }
            }
        });

        let server_handle = tokio::spawn(server);
        info!("Server started successfully");

        let _ = tokio::join!(server_handle, udp_handle);
        info!("Server shutdown complete");

        Ok(())
    }

    #[cfg(test)]
    pub fn get_clients(&self) -> Clients {
        self.clients.clone()
    }
}
