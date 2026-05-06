#![allow(clippy::derivable_impls)]
mod runtime;

use anyhow::anyhow;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf, time::Instant};

use clap::Parser;
#[cfg(feature = "ui")]
use gpui_component::ThemeMode;
pub use runtime::RuntimeTransport;
#[cfg(feature = "ui")]
use tokio::sync::watch;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    controller::{Controller, KeyState, Options as ControllerOptions},
    input::{Key, KeyEvent, Message},
    transport::TransportConnection,
};

#[derive(Clone, Debug, Parser, Serialize, Deserialize)]
pub struct Args {
    #[arg(short, long, default_value_t = Args::default_port())]
    pub port: u16,
    //TODO: test for a good default
    /// Decides what amount of time can pass between clicks to hold (-1 to disable)
    #[arg(long, default_value_t = Args::default_double_tap_timing())]
    pub double_tap_timing: i64,
    /// Sets the postfix that the button has to havbe for it to have double tap to hold. Set to
    /// empty string for all keys
    #[arg(long, default_value_t = String::from(Args::default_double_tap_postfix()))]
    pub double_tap_postfix: String,

    #[command(flatten)]
    pub controller: ControllerOptions,

    #[arg(long, value_enum, default_value_t = TransportKind::default())]
    pub transport: TransportKind,

    #[cfg(all(feature = "bluetooth", target_os = "linux"))]
    #[arg(long, default_value_t = Args::default_bt_channel())]
    pub bt_channel: u8,

    #[arg(long, default_value_t = false)]
    pub disable_tray: bool,
}

impl Args {
    pub const fn default_port() -> u16 {
        1715
    }

    pub const fn default_double_tap_timing() -> i64 {
        200
    }

    pub const fn default_double_tap_postfix() -> &'static str {
        "_dth"
    }

    #[cfg(all(feature = "bluetooth", target_os = "linux"))]
    pub const fn default_bt_channel() -> u8 {
        3
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            double_tap_timing: Self::default_double_tap_timing(),
            double_tap_postfix: String::from(Self::default_double_tap_postfix()),
            controller: ControllerOptions::default(),
            transport: TransportKind::default(),
            #[cfg(all(feature = "bluetooth", target_os = "linux"))]
            bt_channel: Self::default_bt_channel(),
            disable_tray: false,
        }
    }
}

#[derive(Clone, Copy, Debug, clap::ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransportKind {
    #[cfg(feature = "ws")]
    Ws,
    #[cfg(feature = "bluetooth")]
    Bluetooth,
}

#[cfg(feature = "ws")]
impl Default for TransportKind {
    fn default() -> Self {
        Self::Ws
    }
}

#[cfg(all(not(feature = "ws"), feature = "bluetooth"))]
impl Default for TransportKind {
    fn default() -> Self {
        Self::Bluetooth
    }
}

pub struct SettingsManager;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredSettings {
    #[serde(flatten)]
    args: Args,
    #[cfg(feature = "ui")]
    #[serde(default)]
    theme_mode: Option<ThemeMode>,
}

impl SettingsManager {
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("droidpad-gamepad");
        path.push("config.json");
        path
    }

    fn load_stored() -> StoredSettings {
        let path = Self::config_path();
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<StoredSettings>(&content) {
                    Ok(settings) => return settings,
                    Err(err) => warn!("Failed to parse config file at {path:?}: {err}"),
                },
                Err(err) => warn!("Failed to read config file at {path:?}: {err}"),
            }
        }

        StoredSettings {
            args: Args::default(),
            #[cfg(feature = "ui")]
            theme_mode: None,
        }
    }

    fn save_stored(settings: &StoredSettings) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(settings)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn save(args: &Args) -> anyhow::Result<()> {
        let mut settings = Self::load_stored();
        settings.args = args.clone();
        Self::save_stored(&settings)
    }

    #[cfg(feature = "ui")]
    pub fn load_ui_settings(default_theme_mode: ThemeMode) -> (Args, Option<ThemeMode>) {
        let settings = Self::load_stored();
        let theme_mode = settings.theme_mode.or(Some(default_theme_mode));
        (
            settings.args,
            theme_mode.filter(|_| settings.theme_mode.is_some()),
        )
    }

    #[cfg(feature = "ui")]
    pub fn save_theme_mode(theme_mode: ThemeMode) -> anyhow::Result<()> {
        let mut settings = Self::load_stored();
        settings.theme_mode = Some(theme_mode);
        Self::save_stored(&settings)
    }
}

pub fn init_logging() {
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}

pub(crate) async fn start_transport(args: &Args) -> anyhow::Result<RuntimeTransport> {
    args.controller.initialize()?;

    let mut transport = RuntimeTransport::new(args.transport);
    info!("Listening on transport {:?}", args.transport);
    transport.listen(args.clone()).await?;
    info!("Transport {:?} started successfully", args.transport);
    Ok(transport)
}

pub(crate) async fn serve_transport_loop(
    mut transport: RuntimeTransport,
    args: Args,
    #[cfg(feature = "ui")] mut shutdown: Option<watch::Receiver<bool>>,
) -> anyhow::Result<()> {
    loop {
        #[cfg(feature = "ui")]
        {
            let shutdown = shutdown
                .as_mut()
                .expect("shutdown receiver must exist in UI service loop");

            tokio::select! {
                changed = shutdown.changed() => {
                    match changed {
                        Ok(()) if *shutdown.borrow() => {
                            info!("Stopping transport service for {:?}", args.transport);
                            return Ok(());
                        }
                        Ok(()) => {}
                        Err(_) => return Ok(()),
                    }
                }
                accepted = transport.accept(args.clone()) => {
                    match accepted {
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
        }

        #[cfg(not(feature = "ui"))]
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

#[cfg(not(feature = "ui"))]
pub async fn run_cli(args: Args) {
    init_logging();

    if let Err(err) = run_service(args).await {
        error!("{err}");
    }
}

pub async fn handle_connection<C>(mut connection: C, args: Args) -> anyhow::Result<()>
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
    controller_msg: Message,
    device: &mut Controller,
    keys_state: &mut HashMap<u8, KeyState>,
    double_tap_state: &mut HashMap<u8, Instant>,
    args: &Args,
) -> anyhow::Result<()> {
    match controller_msg {
        Message::Dpad {
            id: _,
            button,
            state,
        } => {
            let input = match button.as_str() {
                "LEFT" => Key::DPadLeft(state),
                "RIGHT" => Key::DPadRight(state),
                "UP" => Key::DPadUp(state),
                "DOWN" => Key::DPadDown(state),
                _ => return Err(anyhow!("Unknown DPAD button: {button}")),
            };

            device.write_input(input)?;
        }
        Message::Joystick { id, x, y } => match id.as_str() {
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
        Message::Button { id, state } => {
            let button_id = if args.double_tap_postfix.is_empty() {
                id.as_str()
            } else {
                id.split_once(&args.double_tap_postfix)
                    .map(|(before, _)| before)
                    .unwrap_or(id.as_str())
            };
            let input = match button_id {
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
                return Ok(());
            };

            if !id.ends_with(&args.double_tap_postfix) {
                device.write_input(input)?;
                device.synchronize()?;
                return Ok(());
            }

            let Some(last_time) = double_tap_state.get(&input.into()) else {
                double_tap_state.insert(input.into(), Instant::now());
                keys_state.insert(input.into(), (*key_event).into());
                device.write_input(input)?;
                return Ok(());
            };
            let Some(key_state) = keys_state.get(&input.into()) else {
                // Should not happen if maps are in sync, but handle safely
                keys_state.insert(input.into(), (*key_event).into());
                device.write_input(input)?;
                return Ok(());
            };

            match (key_state, key_event) {
                (KeyState::Pressed, KeyEvent::Release) => {
                    keys_state.insert(input.into(), KeyState::Released);
                    device.write_input(input)?;
                }
                (KeyState::Held, KeyEvent::Release) => {}
                (KeyState::Held, KeyEvent::Press) => {
                    keys_state.insert(input.into(), KeyState::Pressed);
                    device.write_input(input)?;
                }
                (KeyState::Released, KeyEvent::Press) => {
                    if (last_time.elapsed().as_millis() as i64) < args.double_tap_timing {
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
