mod controller;
mod keys;
mod message;

use std::{collections::HashMap, net::SocketAddr, time::Instant};

use axum::{
    Router,
    extract::{
        ConnectInfo, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    response::IntoResponse,
    routing::any,
};
use clap::Parser;
use log::{error, info};
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    controller::{Controller, KeyState},
    keys::Key,
    message::KeyEvent,
};

#[derive(Debug, Parser)]
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

    let router = Router::new().route("/", any(ws_handler)).layer(
        TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)),
    );
    let listener = TcpListener::bind(std::format!("0.0.0.0:{}", args.port))
        .await
        .unwrap();
    info!(
        "Listening on: {}:{}",
        local_ip_address::local_ip()
            .map(|v| v.to_string())
            .unwrap_or(String::from("local_ip")),
        args.port
    );

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}

async fn handle_socket(mut socket: WebSocket, who: SocketAddr) {
    let name = std::format!("droidpad-{}", who.ip());
    match Controller::new(&name) {
        Ok(mut controller) => {
            info!("New controller connected: {name}");
            let mut keys_state: HashMap<u8, KeyState> = HashMap::new();
            let mut double_tap_state: HashMap<u8, Instant> = HashMap::new();
            while let Some(msg) = socket.recv().await {
                let Ok(msg) = msg else {
                    continue;
                };
                if let Message::Close(_) = msg {
                    info!("Controller disconnected: {name}");
                    break;
                };
                let Message::Text(t) = msg else {
                    continue;
                };
                if let Err(err) =
                    handle_messages(&t, &mut controller, &mut keys_state, &mut double_tap_state)
                        .await
                {
                    error!("{err}, {}", t.as_str());
                };
            }
        }
        Err(err) => {
            error!("{err}");
        }
    }
}

async fn handle_messages(
    msg: &Utf8Bytes,
    device: &mut Controller,
    keys_state: &mut HashMap<u8, KeyState>,
    double_tap_state: &mut HashMap<u8, Instant>,
) -> anyhow::Result<()> {
    let controller_msg = serde_json::from_str::<message::Message>(msg)?;

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
            let args = Args::parse();
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
