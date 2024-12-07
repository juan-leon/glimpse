use std::net::SocketAddr;
use tempo::Server;
use tempo::ServerError;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    tracing_subscriber::fmt()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false)
        .init();

    info!("Starting application");

    let ws_addr: SocketAddr = match "127.0.0.1:8099".parse() {
        Ok(addr) => addr,
        Err(e) => {
            error!(?e, "Failed to parse WebSocket address");
            return Err(ServerError::AddrParse(e));
        }
    };

    let udp_addr: SocketAddr = match "127.0.0.1:8081".parse() {
        Ok(addr) => addr,
        Err(e) => {
            error!(?e, "Failed to parse UDP address");
            return Err(ServerError::AddrParse(e));
        }
    };

    let server = Server::new(ws_addr, udp_addr);
    if let Err(e) = server.run().await {
        error!(?e, "Server error");
        return Err(e);
    }

    info!("Application shutdown complete");
    Ok(())
}
