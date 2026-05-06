#[cfg(not(any(feature = "ws", feature = "bluetooth")))]
compile_error!("At least one transport feature must be enabled: `ws` or `bluetooth`.");

#[cfg(any(feature = "ws", feature = "bluetooth"))]
mod app;
mod controller;
mod input;
#[cfg(any(feature = "ws", feature = "bluetooth"))]
mod transport;
#[cfg(all(feature = "ui", any(feature = "ws", feature = "bluetooth")))]
mod ui;

#[cfg(all(not(feature = "ui"), any(feature = "ws", feature = "bluetooth")))]
use clap::Parser;

#[cfg(all(not(feature = "ui"), any(feature = "ws", feature = "bluetooth")))]
use crate::app::{Args, run_cli};

#[cfg(all(not(feature = "ui"), any(feature = "ws", feature = "bluetooth")))]
#[tokio::main]
async fn main() {
    run_cli(Args::parse()).await;
}

#[cfg(all(feature = "ui", any(feature = "ws", feature = "bluetooth")))]
fn main() {
    ui::run();
}

#[cfg(not(any(feature = "ws", feature = "bluetooth")))]
fn main() {}
