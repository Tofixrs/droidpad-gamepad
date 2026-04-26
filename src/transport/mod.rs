#[cfg(all(feature = "bluetooth", target_os = "linux"))]
pub mod bluetooth;
#[cfg(all(feature = "bluetooth", target_os = "windows"))]
pub mod bluetooth_windows;
#[cfg(feature = "ws")]
pub mod ws;

use crate::{Args, message::Message};

pub trait Transport {
    type Connection: TransportConnection;

    async fn listen(&mut self, args: Args) -> anyhow::Result<()>;
    async fn accept(&mut self, args: Args) -> anyhow::Result<Self::Connection>;
}

pub trait TransportConnection {
    fn peer_name(&self) -> String;

    async fn recv_message(&mut self) -> anyhow::Result<Option<Message>>;
}
