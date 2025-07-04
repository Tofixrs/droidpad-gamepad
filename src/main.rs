mod message;
use std::{
    net::SocketAddr,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, anyhow};
use axum::{
    Router,
    extract::{
        ConnectInfo, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    response::IntoResponse,
    routing::any,
};
use evdev_rs::{
    AbsInfo, DeviceWrapper, InputEvent, TimeVal, UInputDevice, UninitDevice,
    enums::{BusType, EV_ABS, EV_KEY, EV_REL, EV_SYN, EventCode},
};
use log::error;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let router = Router::new().route("/", any(ws_handler)).layer(
        TraceLayer::new_for_http().make_span_with(DefaultMakeSpan::default().include_headers(true)),
    );
    let listener = TcpListener::bind("0.0.0.0:1715").await.unwrap();

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
    let mut device = create_controller(std::format!("droidpad-{}", who.ip())).unwrap();
    while let Some(msg) = socket.recv().await {
        let Ok(msg) = msg else {
            continue;
        };
        if let Message::Close(_) = msg {
            break;
        };
        let Message::Text(t) = msg else {
            continue;
        };
        if let Err(err) = handle_messages(&t, &mut device).await {
            error!("{err}, {}", t.as_str());
        };
    }
}

const UINPUT_AXIS_MIN: i32 = -32768;
const UINPUT_AXIS_MAX: i32 = 32767;

fn create_controller(name: String) -> anyhow::Result<UInputDevice> {
    let u = UninitDevice::new().ok_or(anyhow!("Failed to create UninitDevice"))?;
    u.set_name(&name);
    u.set_bustype(BusType::BUS_VIRTUAL as u16);
    u.set_vendor_id(0x045e);
    u.set_product_id(0x028e);

    let abs_info = AbsInfo {
        value: 0,
        minimum: UINPUT_AXIS_MIN,
        maximum: UINPUT_AXIS_MAX,
        fuzz: 0,
        flat: 0,
        resolution: 0,
    };
    u.enable(EventCode::EV_SYN(EV_SYN::SYN_REPORT))?;
    u.enable_event_code(
        &EventCode::EV_ABS(EV_ABS::ABS_X),
        Some(evdev_rs::EnableCodeData::AbsInfo(abs_info)),
    )?;
    u.enable_event_code(
        &EventCode::EV_ABS(EV_ABS::ABS_Y),
        Some(evdev_rs::EnableCodeData::AbsInfo(abs_info)),
    )?;
    u.enable_event_code(
        &EventCode::EV_ABS(EV_ABS::ABS_RX),
        Some(evdev_rs::EnableCodeData::AbsInfo(abs_info)),
    )?;
    u.enable_event_code(
        &EventCode::EV_ABS(EV_ABS::ABS_RY),
        Some(evdev_rs::EnableCodeData::AbsInfo(abs_info)),
    )?;
    u.enable(EventCode::EV_KEY(EV_KEY::BTN_SOUTH))?;
    u.enable(EventCode::EV_KEY(EV_KEY::BTN_EAST))?;
    u.enable(EventCode::EV_KEY(EV_KEY::BTN_NORTH))?;
    u.enable(EventCode::EV_KEY(EV_KEY::BTN_WEST))?;

    u.enable(EventCode::EV_KEY(EV_KEY::BTN_DPAD_UP))?;
    u.enable(EventCode::EV_KEY(EV_KEY::BTN_DPAD_DOWN))?;
    u.enable(EventCode::EV_KEY(EV_KEY::BTN_DPAD_LEFT))?;
    u.enable(EventCode::EV_KEY(EV_KEY::BTN_DPAD_RIGHT))?;

    u.enable(EventCode::EV_KEY(EV_KEY::BTN_TL))?;
    u.enable(EventCode::EV_KEY(EV_KEY::BTN_TL2))?;
    u.enable(EventCode::EV_KEY(EV_KEY::BTN_TR))?;
    u.enable(EventCode::EV_KEY(EV_KEY::BTN_TR2))?;

    u.enable(EventCode::EV_KEY(EV_KEY::BTN_START))?;
    u.enable(EventCode::EV_KEY(EV_KEY::BTN_SELECT))?;

    UInputDevice::create_from_device(&u).with_context(|| "Failed to create uinput device")
}

fn map_float_to_axis_value(f: f32) -> i32 {
    let scaled_value =
        ((f + 1.0) / 2.0) * (UINPUT_AXIS_MAX - UINPUT_AXIS_MIN) as f32 + UINPUT_AXIS_MIN as f32;
    scaled_value.round() as i32
}

async fn handle_messages(msg: &Utf8Bytes, device: &mut UInputDevice) -> anyhow::Result<()> {
    let controller_msg = serde_json::from_str::<message::Message>(msg)?;

    match controller_msg {
        message::Message::Dpad {
            id: _,
            button,
            state,
        } => {
            let input = match button.as_str() {
                "LEFT" => EV_KEY::BTN_DPAD_LEFT,
                "RIGHT" => EV_KEY::BTN_DPAD_RIGHT,
                "UP" => EV_KEY::BTN_DPAD_UP,
                "DOWN" => EV_KEY::BTN_DPAD_DOWN,
                _ => unreachable!(),
            };

            device.write_event(&InputEvent::new(
                &timeval_now(),
                &EventCode::EV_KEY(input),
                state as i32,
            ))?;
        }
        message::Message::Joystick { id, x, y } => match id.as_str() {
            "left" => {
                device.write_event(&InputEvent::new(
                    &timeval_now(),
                    &EventCode::EV_ABS(EV_ABS::ABS_X),
                    map_float_to_axis_value(x),
                ))?;
                device.write_event(&InputEvent::new(
                    &timeval_now(),
                    &EventCode::EV_ABS(EV_ABS::ABS_Y),
                    -map_float_to_axis_value(y),
                ))?;
            }
            "right" => {
                device.write_event(&InputEvent::new(
                    &timeval_now(),
                    &EventCode::EV_ABS(EV_ABS::ABS_RX),
                    map_float_to_axis_value(x),
                ))?;
                device.write_event(&InputEvent::new(
                    &timeval_now(),
                    &EventCode::EV_ABS(EV_ABS::ABS_RY),
                    -map_float_to_axis_value(y),
                ))?;
            }
            _ => {}
        },
        message::Message::Button { id, state } => {
            let input = match id.as_str() {
                "A" => Some(EV_KEY::BTN_SOUTH),
                "B" => Some(EV_KEY::BTN_EAST),
                "X" => Some(EV_KEY::BTN_WEST),
                "Y" => Some(EV_KEY::BTN_NORTH),
                "lb" => Some(EV_KEY::BTN_TL),
                "lt" => Some(EV_KEY::BTN_TL2),
                "rb" => Some(EV_KEY::BTN_TR),
                "rt" => Some(EV_KEY::BTN_TR2),
                "start" => Some(EV_KEY::BTN_START),
                "back" => Some(EV_KEY::BTN_SELECT),
                _ => None,
            };
            let Some(input) = input else {
                return Ok(());
            };

            device.write_event(&InputEvent::new(
                &timeval_now(),
                &EventCode::EV_KEY(input),
                state as i32,
            ))?;
        }
    };
    synchronize(device)?;

    Ok(())
}

fn timeval_now() -> TimeVal {
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH).unwrap();

    let tv_sec = duration_since_epoch.as_secs();
    let tv_usec = duration_since_epoch.subsec_micros();

    TimeVal {
        tv_sec: tv_sec as i64,
        tv_usec: tv_usec as i64,
    }
}
fn synchronize(device: &mut UInputDevice) -> anyhow::Result<()> {
    device.write_event(&InputEvent::new(
        &timeval_now(),
        &EventCode::EV_SYN(EV_SYN::SYN_REPORT),
        0,
    ))?;

    Ok(())
}
