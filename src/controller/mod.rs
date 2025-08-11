#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::*;

use crate::message::KeyEvent;

const DOUBLE_TAP_TIME_MS: u128 = 200;

#[derive(Debug, Default)]
pub enum KeyState {
    Pressed,
    Held,
    #[default]
    Released,
}

impl From<KeyEvent> for KeyState {
    fn from(value: KeyEvent) -> Self {
        match value {
            KeyEvent::Release => KeyState::Released,
            KeyEvent::Press => KeyState::Pressed,
        }
    }
}
