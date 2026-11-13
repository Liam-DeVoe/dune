//! The WebSocket endpoint: one task per socket that moves text between the socket and the game
//! thread. The server pings every 15 s and closes a connection it has heard nothing from for
//! 30 s.

use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use super::http::{AppState, Command};
use super::protocol::MAX_MESSAGE_BYTES;

pub const PING_INTERVAL: Duration = Duration::from_secs(15);
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/// `GET /ws`.
pub async fn handler(upgrade: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    upgrade.max_message_size(MAX_MESSAGE_BYTES).max_frame_size(MAX_MESSAGE_BYTES).on_upgrade(move |socket| run(socket, state))
}

async fn run(socket: WebSocket, state: AppState) {
    let connection = state.next_connection_id();
    let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<String>();
    if state.send(Command::Connected { connection, outgoing: outgoing_tx }).is_err() {
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
                    // The server dropped the connection (a malformed message): close the socket.
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
            _ = &mut writer => break,
            next = tokio::time::timeout(CLIENT_TIMEOUT, stream.next()) => match next {
                Ok(Some(Ok(Message::Text(text)))) => {
                    if state.send(Command::Received { connection, text: text.to_string() }).is_err() {
                        break;
                    }
                }
                Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => {}
                // A binary frame, a close, an error, the end of the stream, or 30 s of silence.
                _ => break,
            },
        }
    }
    let _ = state.send(Command::Disconnected { connection });
    writer.abort();
}
