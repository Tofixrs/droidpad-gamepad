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
