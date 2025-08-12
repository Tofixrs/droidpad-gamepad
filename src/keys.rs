use crate::message::KeyEvent;

#[derive(Copy, Clone)]
pub enum Key {
    LeftJoystickX(f32),
    LeftJoystickY(f32),
    RightJoystickX(f32),
    RightJoystickY(f32),
    ThumbRight(KeyEvent),
    ThumbLeft(KeyEvent),
    DPadUp(KeyEvent),
    DPadDown(KeyEvent),
    DPadLeft(KeyEvent),
    DPadRight(KeyEvent),
    A(KeyEvent),
    B(KeyEvent),
    X(KeyEvent),
    Y(KeyEvent),
    Start(KeyEvent),
    Select(KeyEvent),
    TriggerLeft(KeyEvent),
    BumperLeft(KeyEvent),
    TriggerRight(KeyEvent),
    BumperRight(KeyEvent),
}

impl Key {
    pub fn key_event(&self) -> Option<&KeyEvent> {
        match self {
            Key::LeftJoystickX(_) => None,
            Key::LeftJoystickY(_) => None,
            Key::RightJoystickX(_) => None,
            Key::RightJoystickY(_) => None,
            Key::DPadUp(state) => Some(state),
            Key::DPadDown(state) => Some(state),
            Key::DPadLeft(state) => Some(state),
            Key::DPadRight(state) => Some(state),
            Key::A(state) => Some(state),
            Key::B(state) => Some(state),
            Key::X(state) => Some(state),
            Key::Y(state) => Some(state),
            Key::Start(state) => Some(state),
            Key::Select(state) => Some(state),
            Key::TriggerLeft(state) => Some(state),
            Key::BumperLeft(state) => Some(state),
            Key::TriggerRight(state) => Some(state),
            Key::BumperRight(state) => Some(state),
            Key::ThumbRight(key_event) => Some(key_event),
            Key::ThumbLeft(key_event) => Some(key_event),
        }
    }
}

impl From<Key> for u8 {
    fn from(k: Key) -> u8 {
        match k {
            Key::LeftJoystickX(_) => 0,
            Key::LeftJoystickY(_) => 1,
            Key::RightJoystickX(_) => 2,
            Key::RightJoystickY(_) => 3,
            Key::DPadUp(_) => 4,
            Key::DPadDown(_) => 5,
            Key::DPadLeft(_) => 6,
            Key::DPadRight(_) => 7,
            Key::A(_) => 8,
            Key::B(_) => 9,
            Key::X(_) => 10,
            Key::Y(_) => 11,
            Key::Start(_) => 12,
            Key::Select(_) => 13,
            Key::TriggerLeft(_) => 14,
            Key::BumperLeft(_) => 15,
            Key::TriggerRight(_) => 16,
            Key::BumperRight(_) => 17,
            Key::ThumbRight(_) => 18,
            Key::ThumbLeft(_) => 19,
        }
    }
}
