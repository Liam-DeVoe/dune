//! The WebSocket endpoint: one task per socket that moves text between the socket and the hub
//! thread. Keepalive and size limits are in protocol.md, "Connection": a ping every 15 s, and
//! the connection closes when nothing has arrived for 30 s (SignalR's defaults).

use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::runtime::{HubCommand, HubHandle};

/// `KeepAliveInterval`.
pub const PING_INTERVAL: Duration = Duration::from_secs(15);
/// `ClientTimeoutInterval`.
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// `GET /ws`.
pub async fn handler(upgrade: WebSocketUpgrade, State(hub): State<HubHandle>) -> Response {
    upgrade.max_message_size(super::MAX_MESSAGE_BYTES).max_frame_size(super::MAX_MESSAGE_BYTES).on_upgrade(move |socket| run(socket, hub))
}

async fn run(socket: WebSocket, hub: HubHandle) {
    let connection = hub.next_connection_id();
    let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<String>();
    if hub.send(HubCommand::Connected { connection, outgoing: outgoing_tx }).is_err() {
        return;
    }
    let (mut sink, mut stream) = socket.split();
    let mut writer = tokio::spawn(async move {
        let mut ping = tokio::time::interval_at(tokio::time::Instant::now() + PING_INTERVAL, PING_INTERVAL);
        loop {
            tokio::select! {
                text = outgoing_rx.recv() => match text {
                    Some(text) => {
                        if sink.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    // The hub dropped the connection (a malformed message): close the socket.
                    None => {
                        let _ = sink.send(Message::Close(None)).await;
                        break;
                    }
                },
                _ = ping.tick() => {
                    if sink.send(Message::Ping(Vec::new().into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
    loop {
        tokio::select! {
            // The writer ends when the hub closed the connection or the socket failed.
            _ = &mut writer => break,
            next = tokio::time::timeout(CLIENT_TIMEOUT, stream.next()) => match next {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if hub.send(HubCommand::Received { connection, text: text.to_string() }).is_err() {
                        break;
                    }
                }
                // Pings and pongs only show that the client is alive.
                Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {}
                // A binary frame, a close, an error, the end of the stream, or 30 s of silence.
                _ => break,
            },
        }
    }
    let _ = hub.send(HubCommand::Disconnected { connection });
    writer.abort();
}
