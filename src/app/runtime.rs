#[cfg(feature = "bluetooth")]
use crate::transport::bluetooth::{BluetoothTransport, BluetoothTransportConnection};
#[cfg(feature = "ws")]
use crate::transport::ws::{WsTransport, WsTransportConnection};
use crate::{
    app::{Args, TransportKind},
    input::Message,
    transport::{Transport, TransportConnection},
};

pub enum RuntimeTransport {
    #[cfg(feature = "ws")]
    Ws(WsTransport),
    #[cfg(feature = "bluetooth")]
    Bluetooth(BluetoothTransport),
}

impl RuntimeTransport {
    pub fn new(kind: TransportKind) -> Self {
        match kind {
            #[cfg(feature = "ws")]
            TransportKind::Ws => Self::Ws(WsTransport::new()),
            #[cfg(feature = "bluetooth")]
            TransportKind::Bluetooth => Self::Bluetooth(BluetoothTransport::new()),
        }
    }

    pub async fn listen(&mut self, args: Args) -> anyhow::Result<()> {
        match self {
            #[cfg(feature = "ws")]
            Self::Ws(transport) => transport.listen(args).await,
            #[cfg(feature = "bluetooth")]
            Self::Bluetooth(transport) => transport.listen(args).await,
        }
    }

    pub async fn accept(&mut self, args: Args) -> anyhow::Result<RuntimeConnection> {
        match self {
            #[cfg(feature = "ws")]
            Self::Ws(transport) => Ok(RuntimeConnection::Ws(transport.accept(args).await?)),
            #[cfg(feature = "bluetooth")]
            Self::Bluetooth(transport) => {
                Ok(RuntimeConnection::Bluetooth(transport.accept(args).await?))
            }
        }
    }
}

#[allow(clippy::large_enum_variant)]
pub enum RuntimeConnection {
    #[cfg(feature = "ws")]
    Ws(WsTransportConnection),
    #[cfg(feature = "bluetooth")]
    Bluetooth(BluetoothTransportConnection),
}

impl TransportConnection for RuntimeConnection {
    fn peer_name(&self) -> String {
        match self {
            #[cfg(feature = "ws")]
            Self::Ws(connection) => connection.peer_name(),
            #[cfg(feature = "bluetooth")]
            Self::Bluetooth(connection) => connection.peer_name(),
        }
    }

    async fn recv_message(&mut self) -> anyhow::Result<Option<Message>> {
        match self {
            #[cfg(feature = "ws")]
            Self::Ws(connection) => connection.recv_message().await,
            #[cfg(feature = "bluetooth")]
            Self::Bluetooth(connection) => connection.recv_message().await,
        }
    }
}
