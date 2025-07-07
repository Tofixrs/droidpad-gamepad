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
use log::{error, info};
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{controller::Controller, keys::Keys};

#[tokio::main]
async fn main() {
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
    let listener = TcpListener::bind("0.0.0.0:1715").await.unwrap();
    info!(
        "Listening on: {}:1715",
        local_ip_address::local_ip()
            .map(|v| v.to_string())
            .unwrap_or(String::from("local_ip"))
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
                "LEFT" => Keys::DPadLeft(state),
                "RIGHT" => Keys::DPadRight(state),
                "UP" => Keys::DPadUp(state),
                "DOWN" => Keys::DPadDown(state),
                _ => unreachable!(),
            };

            device.send_key(input)?;
        }
        message::Message::Joystick { id, x, y } => match id.as_str() {
            "left" => {
                device.send_key(Keys::LeftJoystickX(x))?;
                device.send_key(Keys::LeftJoystickY(y))?;
            }
            "right" => {
                device.send_key(Keys::RightJoystickX(x))?;
                device.send_key(Keys::RightJoystickY(y))?;
            }
            _ => {}
        },
        message::Message::Button { id, state } => {
            let input = match id.as_str() {
                "A" => Some(Keys::A(state)),
                "B" => Some(Keys::B(state)),
                "X" => Some(Keys::X(state)),
                "Y" => Some(Keys::Y(state)),
                "lb" => Some(Keys::BumperLeft(state)),
                "lt" => Some(Keys::TriggerLeft(state)),
                "rb" => Some(Keys::BumperRight(state)),
                "rt" => Some(Keys::TriggerRight(state)),
                "start" => Some(Keys::Start(state)),
                "back" => Some(Keys::Select(state)),
                _ => None,
            };
            let Some(input) = input else {
                return Ok(());
            };
            device.send_key(input)?;
        }
    };
    device.synchronize()?;

    Ok(())
}
