#[cfg(feature = "bluetooth")]
pub mod bluetooth;
#[cfg(feature = "ws")]
pub mod ws;

use crate::{app::Args, input::Message};

pub trait Transport {
    type Connection: TransportConnection;

    async fn listen(&mut self, args: Args) -> anyhow::Result<()>;
    async fn accept(&mut self, args: Args) -> anyhow::Result<Self::Connection>;
}

pub trait TransportConnection {
    fn peer_name(&self) -> String;

    async fn recv_message(&mut self) -> anyhow::Result<Option<Message>>;
}
