use std::net::SocketAddr;

use anyhow::{Context, anyhow};
use futures_util::StreamExt;
use log::info;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message as WsMessage};

use super::{Transport, TransportConnection};
use crate::{app::Args, input::Message};

#[derive(Default)]
pub struct WsTransport {
    listener: Option<TcpListener>,
}

impl WsTransport {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Transport for WsTransport {
    type Connection = WsTransportConnection;

    async fn listen(&mut self, args: Args) -> anyhow::Result<()> {
        if self.listener.is_some() {
            return Ok(());
        }

        let listener = TcpListener::bind(std::format!("0.0.0.0:{}", args.port)).await?;
        info!(
            "Listening on: {}:{}",
            local_ip_address::local_ip()
                .map(|v| v.to_string())
                .unwrap_or(String::from("local_ip")),
            args.port
        );
        self.listener = Some(listener);

        Ok(())
    }

    async fn accept(&mut self, _args: Args) -> anyhow::Result<Self::Connection> {
        let listener = self
            .listener
            .as_mut()
            .ok_or_else(|| anyhow!("WebSocket transport is not listening"))?;
        let (stream, peer_addr) = listener.accept().await?;
        let socket = accept_async(stream).await?;

        Ok(WsTransportConnection::new(socket, peer_addr))
    }
}

pub struct WsTransportConnection {
    socket: WebSocketStream<TcpStream>,
    peer_addr: SocketAddr,
}

impl WsTransportConnection {
    pub fn new(socket: WebSocketStream<TcpStream>, peer_addr: SocketAddr) -> Self {
        Self { socket, peer_addr }
    }
}

impl TransportConnection for WsTransportConnection {
    fn peer_name(&self) -> String {
        format!("droidpad-{}", self.peer_addr.ip())
    }

    async fn recv_message(&mut self) -> anyhow::Result<Option<Message>> {
        loop {
            let Some(message) = self.socket.next().await else {
                return Ok(None);
            };
            let message = message?;

            match message {
                WsMessage::Text(text) => {
                    let parsed = serde_json::from_str::<Message>(text.as_ref())
                        .context("Failed to parse websocket message as DroidPad JSON")?;
                    return Ok(Some(parsed));
                }
                WsMessage::Close(_) => return Ok(None),
                WsMessage::Ping(_) | WsMessage::Pong(_) => continue,
                WsMessage::Binary(_) => {
                    return Err(anyhow!("Binary websocket messages are not supported"));
                }
                WsMessage::Frame(_) => continue,
            }
        }
    }
}
