#[cfg(any(target_os = "linux", target_os = "windows"))]
use anyhow::{Context, anyhow};
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(alias = "DPAD")]
    Dpad {
        id: String,
        button: String,
        state: KeyEvent,
    },
    #[serde(alias = "JOYSTICK")]
    Joystick { id: String, x: f32, y: f32 },
    #[serde(alias = "BUTTON")]
    Button { id: String, state: KeyEvent },
}

#[repr(u8)]
#[derive(Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum KeyEvent {
    #[serde(alias = "RELEASE")]
    Release = 0,
    #[serde(alias = "PRESS")]
    Press = 1,
}

impl Message {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub fn from_droidpad_csv(line: &str) -> anyhow::Result<Self> {
        let parts: Vec<_> = line.trim().split(',').collect();
        if parts.is_empty() || parts[0].is_empty() {
            return Err(anyhow!("Empty DroidPad CSV message"));
        }

        match parts.as_slice() {
            [id, "BUTTON", state] => Ok(Self::Button {
                id: (*id).to_string(),
                state: KeyEvent::from_droidpad_csv(state)?,
            }),
            [id, "DPAD", button, state] => Ok(Self::Dpad {
                id: (*id).to_string(),
                button: (*button).to_string(),
                state: KeyEvent::from_droidpad_csv(state)?,
            }),
            [id, "JOYSTICK", x, y] => Ok(Self::Joystick {
                id: (*id).to_string(),
                x: x.parse().context("Invalid joystick x value")?,
                y: y.parse().context("Invalid joystick y value")?,
            }),
            _ => Err(anyhow!("Unsupported DroidPad CSV message: {line}")),
        }
    }
}

impl KeyEvent {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    pub fn from_droidpad_csv(value: &str) -> anyhow::Result<Self> {
        match value {
            "PRESS" => Ok(Self::Press),
            "RELEASE" => Ok(Self::Release),
            "CLICK" => Err(anyhow!("DroidPad CLICK events are not supported yet")),
            _ => Err(anyhow!("Unknown DroidPad key event: {value}")),
        }
    }
}

impl From<KeyEvent> for bool {
    fn from(value: KeyEvent) -> Self {
        match value {
            KeyEvent::Release => false,
            KeyEvent::Press => true,
        }
    }
}
