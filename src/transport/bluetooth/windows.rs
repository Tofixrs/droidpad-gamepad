use anyhow::{Context, anyhow};
use log::info;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use windows::{
    Devices::Bluetooth::Rfcomm::{RfcommServiceId, RfcommServiceProvider},
    Foundation::TypedEventHandler,
    Networking::Sockets::{
        StreamSocket, StreamSocketListener, StreamSocketListenerConnectionReceivedEventArgs,
    },
    Storage::Streams::{DataReader, InputStreamOptions},
};

use crate::{
    app::Args,
    input::Message,
    transport::{Transport, TransportConnection},
};

pub struct BluetoothTransport {
    listener: Option<StreamSocketListener>,
    provider: Option<RfcommServiceProvider>,
    receiver: Option<UnboundedReceiver<StreamSocket>>,
    connection_received_token: Option<i64>,
}

impl BluetoothTransport {
    pub fn new() -> Self {
        Self {
            listener: None,
            provider: None,
            receiver: None,
            connection_received_token: None,
        }
    }
}

impl Transport for BluetoothTransport {
    type Connection = BluetoothTransportConnection;

    async fn listen(&mut self, _args: Args) -> anyhow::Result<()> {
        if self.listener.is_some() {
            info!("Windows Bluetooth RFCOMM listener already running");
            return Ok(());
        }

        info!("Starting Windows Bluetooth RFCOMM transport");
        let provider = RfcommServiceProvider::CreateAsync(&RfcommServiceId::SerialPort()?)
            .context("Failed to create Windows Bluetooth RFCOMM service provider")?
            .await
            .context("Failed to initialize Windows Bluetooth RFCOMM service provider")?;
        info!("Created Windows Bluetooth RFCOMM service provider");
        let listener = StreamSocketListener::new()
            .context("Failed to create Windows Bluetooth socket listener")?;
        info!("Created Windows Bluetooth socket listener");
        let (sender, receiver) = unbounded_channel();

        let connection_received = TypedEventHandler::<
            StreamSocketListener,
            StreamSocketListenerConnectionReceivedEventArgs,
        >::new(move |_, args| {
            let socket = args.ok()?.Socket()?;
            let _ = sender.send(socket);
            Ok(())
        });
        let token = listener
            .ConnectionReceived(&connection_received)
            .context("Failed to subscribe to Windows Bluetooth connection events")?;
        info!("Registered Windows Bluetooth connection handler token {token}");

        let service_name = provider
            .ServiceId()
            .context("Failed to get Windows Bluetooth service ID")?
            .AsString()
            .context("Failed to stringify Windows Bluetooth service ID")?;
        listener
            .BindServiceNameAsync(&service_name)
            .context("Failed to bind Windows Bluetooth service name")?
            .await
            .context("Failed while waiting for Windows Bluetooth service bind")?;
        info!("Bound Windows Bluetooth service name: {service_name}");
        provider
            .StartAdvertisingWithRadioDiscoverability(&listener, true)
            .context("Failed to start Windows Bluetooth advertising")?;
        info!("Started Windows Bluetooth advertising as discoverable");

        self.connection_received_token = Some(token);
        self.receiver = Some(receiver);
        self.provider = Some(provider);
        self.listener = Some(listener);

        Ok(())
    }

    async fn accept(&mut self, _args: Args) -> anyhow::Result<Self::Connection> {
        let receiver = self
            .receiver
            .as_mut()
            .ok_or_else(|| anyhow!("Bluetooth transport is not listening. Ensure Windows Bluetooth is enabled and the application has permissions."))?;
        info!("Waiting for a Windows Bluetooth connection");
        let socket = receiver
            .recv()
            .await
            .ok_or_else(|| anyhow!("Bluetooth listener connection channel closed unexpectedly."))?;
        info!("Windows Bluetooth connection received");

        BluetoothTransportConnection::new(socket)
    }
}

impl Drop for BluetoothTransport {
    fn drop(&mut self) {
        if let Some(provider) = &self.provider {
            provider.StopAdvertising().ok();
        }

        if let (Some(listener), Some(token)) = (&self.listener, self.connection_received_token) {
            listener.RemoveConnectionReceived(token).ok();
        }
    }
}

pub struct BluetoothTransportConnection {
    socket: StreamSocket,
    reader: DataReader,
}

impl BluetoothTransportConnection {
    fn new(socket: StreamSocket) -> anyhow::Result<Self> {
        let reader = DataReader::CreateDataReader(&socket.InputStream()?)?;
        reader.SetInputStreamOptions(InputStreamOptions::Partial)?;

        Ok(Self { socket, reader })
    }
}

impl TransportConnection for BluetoothTransportConnection {
    fn peer_name(&self) -> String {
        self.socket
            .Information()
            .ok()
            .and_then(|info| info.RemoteAddress().ok())
            .map(|host| format!("droidpad-{}", host.DisplayName().unwrap_or_default()))
            .unwrap_or_else(|| String::from("droidpad-bluetooth"))
    }

    async fn recv_message(&mut self) -> anyhow::Result<Option<Message>> {
        let mut line = Vec::new();

        loop {
            let loaded = self.reader.LoadAsync(1)?.await?;
            if loaded == 0 {
                if line.is_empty() {
                    return Ok(None);
                }
                break;
            }

            let byte = self.reader.ReadByte()?;
            match byte {
                b'\n' => break,
                b'\r' | 0 => continue,
                other => line.push(other),
            }
        }

        let line = String::from_utf8(line).context("Bluetooth message was not valid UTF-8")?;
        if line.is_empty() {
            return Ok(None);
        }

        Message::from_droidpad_csv(&line)
            .context("Failed to parse Windows Bluetooth RFCOMM message")
            .map(Some)
    }
}
