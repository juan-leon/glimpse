use crate::types::Clients;
use futures::{SinkExt, StreamExt};
use tokio::select;
use tokio::sync::broadcast;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{error, info, instrument};
use warp::ws::{Message, WebSocket};

#[instrument(skip(ws, clients, shutdown))]
pub async fn handle_websocket_client(
    ws: WebSocket,
    clients: Clients,
    mut shutdown: broadcast::Receiver<()>,
) {
    info!("New WebSocket connection established");
    let (mut ws_tx, mut ws_rx) = ws.split();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let mut rx = UnboundedReceiverStream::new(rx);
    let client_id = uuid::Uuid::new_v4();

    clients.lock().await.insert(client_id.to_string(), tx);

    let ws_send_task = tokio::spawn(async move {
        while let Some(message) = rx.next().await {
            if let Err(e) = ws_tx.send(message).await {
                error!(?e, "WebSocket send error");
                break;
            }
        }
    });

    let mut ws_receive_task = tokio::spawn(async move {
        while let Some(result) = ws_rx.next().await {
            match result {
                Ok(msg) => {
                    info!(?msg, ?client_id, "Received WebSocket message");
                }
                Err(e) => {
                    error!(?client_id, ?e, "WebSocket receive error");
                    break;
                }
            }
        }
    });

    select! {
        _ = &mut ws_receive_task => {
            info!(?client_id, "WebSocket connection closed by client");
        }
        _ = shutdown.recv() => {
            info!(?client_id, "WebSocket connection closed by shutdown");
        }
    }

    ws_send_task.abort();
    ws_receive_task.abort();
    clients.lock().await.remove(&client_id.to_string());
}
