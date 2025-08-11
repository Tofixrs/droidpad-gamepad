mod controller;
mod keys;
mod message;

use std::net::SocketAddr;

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

use crate::{controller::Controller, keys::Key};

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long, default_value_t = 1715)]
    port: u16,
    //TODO: test for a good default
    /// Decides what amount of time can pass between clicks to hold (-1 to disable)
    #[arg(short, long, default_value_t = 200)]
    double_tap_timing: i128,
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
                if let Err(err) = handle_messages(&t, &mut controller).await {
                    error!("{err}, {}", t.as_str());
                };
            }
        }
        Err(err) => {
            error!("{err}");
        }
    }
}

async fn handle_messages(msg: &Utf8Bytes, device: &mut Controller) -> anyhow::Result<()> {
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

            device.handle_key(input)?;
        }
        message::Message::Joystick { id, x, y } => match id.as_str() {
            "left" => {
                device.handle_key(Key::LeftJoystickX(x))?;
                device.handle_key(Key::LeftJoystickY(y))?;
            }
            "right" => {
                device.handle_key(Key::RightJoystickX(x))?;
                device.handle_key(Key::RightJoystickY(y))?;
            }
            _ => {}
        },
        message::Message::Button { id, state } => {
            let input = match id.as_str() {
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
                _ => None,
            };
            let Some(input) = input else {
                return Ok(());
            };
            device.handle_key(input)?;
        }
    };
    device.synchronize()?;

    Ok(())
}
