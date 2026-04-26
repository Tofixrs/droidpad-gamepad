use anyhow::{Context, anyhow};
use bluer::{
    Address, Session, Uuid, UuidExt,
    rfcomm::{Profile, ProfileHandle, Role, Stream},
};
use futures_util::StreamExt;
use log::info;
use tokio::io::{AsyncBufReadExt, BufReader};

use super::{Transport, TransportConnection};
use crate::{Args, message::Message};

pub struct BluetoothTransport {
    profile_handle: Option<ProfileHandle>,
}

impl BluetoothTransport {
    pub fn new() -> Self {
        Self { profile_handle: None }
    }
}

impl Transport for BluetoothTransport {
    type Connection = BluetoothTransportConnection;

    async fn listen(&mut self, args: Args) -> anyhow::Result<()> {
        if self.profile_handle.is_some() {
            return Ok(());
        }

        let serial_port_uuid = Uuid::from_u16(0x1101);
        let session = Session::new()
            .await
            .context("Failed to connect to the BlueZ session")?;
        let adapter = session
            .default_adapter()
            .await
            .context("Failed to resolve the default Bluetooth adapter")?;
        adapter
            .set_powered(true)
            .await
            .context("Failed to power on the Bluetooth adapter")?;
        adapter
            .set_discoverable(true)
            .await
            .context("Failed to make the Bluetooth adapter discoverable")?;
        let profile_handle = session
            .register_profile(Profile {
                uuid: serial_port_uuid,
                name: Some(String::from("DroidPad")),
                service: Some(serial_port_uuid),
                role: Some(Role::Server),
                channel: Some(args.bt_channel.into()),
                require_authentication: Some(false),
                require_authorization: Some(false),
                ..Default::default()
            })
            .await
            .with_context(|| {
                format!(
                    "Failed to register Bluetooth Serial Port profile on channel {}",
                    args.bt_channel
                )
            })?;
        info!(
            "Advertising Bluetooth Serial Port service {} on channel {}",
            serial_port_uuid, args.bt_channel
        );
        self.profile_handle = Some(profile_handle);

        Ok(())
    }

    async fn accept(&mut self, _args: Args) -> anyhow::Result<Self::Connection> {
        let profile_handle = self
            .profile_handle
            .as_mut()
            .ok_or_else(|| anyhow!("Bluetooth transport is not listening"))?;
        let request = profile_handle
            .next()
            .await
            .ok_or_else(|| anyhow!("Bluetooth profile connection stream ended"))?;
        let peer_addr = request.device();
        let stream = request
            .accept()
            .context("Failed to accept Bluetooth profile connection")?;

        Ok(BluetoothTransportConnection::new(stream, peer_addr))
    }
}

pub struct BluetoothTransportConnection {
    reader: BufReader<Stream>,
    peer_addr: Address,
}

impl BluetoothTransportConnection {
    fn new(stream: Stream, peer_addr: Address) -> Self {
        Self {
            reader: BufReader::new(stream),
            peer_addr,
        }
    }
}

impl TransportConnection for BluetoothTransportConnection {
    fn peer_name(&self) -> String {
        format!("droidpad-{}", self.peer_addr)
    }

    async fn recv_message(&mut self) -> anyhow::Result<Option<Message>> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            return Ok(None);
        }

        let line = line.trim_matches(|c| c == '\r' || c == '\n' || c == '\0');
        if line.is_empty() {
            return Ok(None);
        }

        Message::from_droidpad_csv(line)
            .context("Failed to parse Bluetooth RFCOMM message")
            .map(Some)
    }
}
