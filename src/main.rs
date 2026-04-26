mod controller;
mod keys;
mod message;
mod transport;

use std::{collections::HashMap, time::Instant};

use clap::Parser;
use log::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    controller::{Controller, KeyState, Options as ControllerOptions},
    keys::Key,
    message::KeyEvent,
    transport::{Transport, TransportConnection},
};
#[cfg(feature = "ws")]
use crate::transport::ws::{WsTransport, WsTransportConnection};
#[cfg(all(feature = "bluetooth", target_os = "linux"))]
use crate::transport::bluetooth::{BluetoothTransport, BluetoothTransportConnection};
#[cfg(all(feature = "bluetooth", target_os = "windows"))]
use crate::transport::bluetooth_windows::{BluetoothTransport, BluetoothTransportConnection};

#[derive(Clone, Debug, Parser)]
struct Args {
    #[arg(short, long, default_value_t = 1715)]
    port: u16,
    //TODO: test for a good default
    /// Decides what amount of time can pass between clicks to hold (-1 to disable)
    #[arg(long, default_value_t = 200)]
    double_tap_timing: i128,
    /// Sets the postfix that the button has to havbe for it to have double tap to hold. Set to
    /// empty string for all keys
    #[arg(long, default_value_t = String::from("_dth"))]
    double_tap_postfix: String,

    #[command(flatten)]
    controller: ControllerOptions,

    #[arg(long, value_enum, default_value_t = default_transport_kind())]
    transport: TransportKind,

    #[cfg(all(feature = "bluetooth", target_os = "linux"))]
    #[arg(long, default_value_t = 3)]
    bt_channel: u8,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum TransportKind {
    #[cfg(feature = "ws")]
    Ws,
    #[cfg(feature = "bluetooth")]
    Bluetooth,
}

const fn default_transport_kind() -> TransportKind {
    #[cfg(feature = "ws")]
    {
        TransportKind::Ws
    }

    #[cfg(all(not(feature = "ws"), feature = "bluetooth"))]
    {
        TransportKind::Bluetooth
    }
}

enum RuntimeTransport {
    #[cfg(feature = "ws")]
    Ws(WsTransport),
    #[cfg(feature = "bluetooth")]
    Bluetooth(BluetoothTransport),
}

impl RuntimeTransport {
    fn new(kind: TransportKind) -> Self {
        match kind {
            #[cfg(feature = "ws")]
            TransportKind::Ws => Self::Ws(WsTransport::new()),
            #[cfg(feature = "bluetooth")]
            TransportKind::Bluetooth => Self::Bluetooth(BluetoothTransport::new()),
        }
    }

    async fn listen(&mut self, args: Args) -> anyhow::Result<()> {
        match self {
            #[cfg(feature = "ws")]
            Self::Ws(transport) => transport.listen(args).await,
            #[cfg(feature = "bluetooth")]
            Self::Bluetooth(transport) => transport.listen(args).await,
        }
    }

    async fn accept(&mut self, args: Args) -> anyhow::Result<RuntimeConnection> {
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

enum RuntimeConnection {
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

    async fn recv_message(&mut self) -> anyhow::Result<Option<message::Message>> {
        match self {
            #[cfg(feature = "ws")]
            Self::Ws(connection) => connection.recv_message().await,
            #[cfg(feature = "bluetooth")]
            Self::Bluetooth(connection) => connection.recv_message().await,
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    if let Err(err) = args.controller.initialize() {
        error!("Failed to initialize controller backend: {err}");
    }

    let mut transport = RuntimeTransport::new(args.transport);
    if let Err(err) = transport.listen(args.clone()).await {
        error!("Failed to initialize transport listener: {err}");
        return;
    }

    loop {
        match transport.accept(args.clone()).await {
            Ok(connection) => {
                let args = args.clone();
                tokio::spawn(async move {
                    if let Err(err) = handle_connection(connection, args).await {
                        error!("{err}");
                    }
                });
            }
            Err(err) => {
                error!("Failed to accept transport connection: {err}");
            }
        }
    }
}

async fn handle_connection<C>(mut connection: C, args: Args) -> anyhow::Result<()>
where
    C: TransportConnection + Send + 'static,
{
    let name = connection.peer_name();

    match Controller::new(&name, &args.controller) {
        Ok(mut controller) => {
            info!("New controller connected: {name}");
            let mut keys_state: HashMap<u8, KeyState> = HashMap::new();
            let mut double_tap_state: HashMap<u8, Instant> = HashMap::new();

            while let Some(message) = connection.recv_message().await? {
                handle_message(
                    message,
                    &mut controller,
                    &mut keys_state,
                    &mut double_tap_state,
                    &args,
                )
                .await?;
            }

            info!("Controller disconnected: {name}");
            Ok(())
        }
        Err(err) => Err(err),
    }
}

async fn handle_message(
    controller_msg: message::Message,
    device: &mut Controller,
    keys_state: &mut HashMap<u8, KeyState>,
    double_tap_state: &mut HashMap<u8, Instant>,
    args: &Args,
) -> anyhow::Result<()> {
    match controller_msg {
        message::Message::Dpad {
            id: _,
            button,
            state,
        } => {
            let input = match button.as_str() {
                "LEFT" => Key::DPadLeft(state),
                "RIGHT" => Key::DPadRight(state),
                "UP" => Key::DPadUp(state),
                "DOWN" => Key::DPadDown(state),
                _ => unreachable!(),
            };

            device.write_input(input)?;
        }
        message::Message::Joystick { id, x, y } => match id.as_str() {
            "left" => {
                device.write_input(Key::LeftJoystickX(x))?;
                device.write_input(Key::LeftJoystickY(y))?;
            }
            "right" => {
                device.write_input(Key::RightJoystickX(x))?;
                device.write_input(Key::RightJoystickY(y))?;
            }
            _ => {}
        },
        message::Message::Button { id, state } => {
            let input = match id
                .split_once(&args.double_tap_postfix)
                .map(|(before, _)| before)
                .unwrap_or(&id)
            {
                "A" => Some(Key::A(state)),
                "B" => Some(Key::B(state)),
                "X" => Some(Key::X(state)),
                "Y" => Some(Key::Y(state)),
                "lb" => Some(Key::BumperLeft(state)),
                "lt" => Some(Key::TriggerLeft(state)),
                "rb" => Some(Key::BumperRight(state)),
                "rt" => Some(Key::TriggerRight(state)),
                "start" => Some(Key::Start(state)),
                "back" => Some(Key::Select(state)),
                "thumb_right" => Some(Key::ThumbRight(state)),
                "thumb_left" => Some(Key::ThumbLeft(state)),
                _ => None,
            };
            let Some(input) = input else {
                return Ok(());
            };

            let Some(key_event) = input.key_event() else {
                //we ignore joysticks; we handle them earlier
                return Ok(());
            };

            if !id.ends_with(&args.double_tap_postfix) {
                device.write_input(input)?;
                device.synchronize()?;
                return Ok(());
            };

            let Some(last_time) = double_tap_state.get(&input.into()) else {
                // this key wasnt registered yet we dont care to check if double clicked
                double_tap_state.insert(input.into(), Instant::now());
                keys_state.insert(input.into(), (*key_event).into());
                device.write_input(input)?;
                return Ok(());
            };
            //this will never fail (i think lol). We always insert key state in the last let else
            let key_state = keys_state.get(&input.into()).unwrap();

            match (key_state, key_event) {
                (KeyState::Pressed, KeyEvent::Release) => {
                    keys_state.insert(input.into(), KeyState::Released);
                    device.write_input(input)?;
                }
                // dont do anythin cuz we just started holdin
                (KeyState::Held, KeyEvent::Release) => {}
                (KeyState::Held, KeyEvent::Press) => {
                    keys_state.insert(input.into(), KeyState::Pressed);
                    device.write_input(input)?;
                }
                (KeyState::Released, KeyEvent::Press) => {
                    if (last_time.elapsed().as_millis() as i128) < args.double_tap_timing {
                        keys_state.insert(input.into(), KeyState::Held);
                        device.write_input(input)?;
                    } else {
                        keys_state.insert(input.into(), KeyState::Pressed);
                        double_tap_state.insert(input.into(), Instant::now());
                        device.write_input(input)?;
                    }
                }
                (KeyState::Released, KeyEvent::Release) => {}
                (KeyState::Pressed, KeyEvent::Press) => {}
            }
        }
    };
    device.synchronize()?;

    Ok(())
}
