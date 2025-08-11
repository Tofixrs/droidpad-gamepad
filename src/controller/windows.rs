use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    time::Instant,
};

use log::info;
use vjoy::{ButtonState, Device, VJoy};

use crate::{
    controller::{DOUBLE_TAP_TIME_MS, KeyState},
    keys::Key,
    message::KeyEvent,
};
use anyhow::anyhow;

pub struct Controller {
    device: Device,
    keys_state: HashMap<u8, KeyState>,
    double_tap_state: HashMap<u8, Instant>,
}

static VJOY_ID: LazyLock<Mutex<u8>> = LazyLock::new(|| Mutex::new(1));
static VJOY: LazyLock<Mutex<VJoy>> =
    LazyLock::new(|| Mutex::new(VJoy::from_default_dll_location().expect("Failed to init vjoy")));

impl Controller {
    //device_name only here so its easier to do multi platform
    pub fn new(_device_name: &str) -> anyhow::Result<Self> {
        let Ok(vjoy) = VJOY.lock() else {
            return Err(anyhow!("Failed to get vjoy"));
        };

        let Ok(mut device_id) = VJOY_ID.lock() else {
            return Err(anyhow!("Failed to get device id"));
        };
        let device = vjoy.get_device_state(device_id.clone() as u32)?;
        info!("Connecting vjoy device {device_id}");
        *device_id += 1;
        Ok(Self {
            device,
            keys_state: HashMap::default(),
            double_tap_state: HashMap::default(),
        })
    }
    pub fn handle_key(&mut self, key: Key) -> anyhow::Result<()> {
        let t: (u8, Value) = key.into();

        let mut write_event = |v: (u8, Value)| -> anyhow::Result<()> {
            match v {
                (axis, Value::Axis(v)) => self.device.set_axis(axis as u32, v),
                (key, Value::Button(state)) => self.device.set_button(key, state),
            }?;

            Ok(())
        };
        let Some(key_event) = key.key_event() else {
            //we ignore joysticks; they dont have btn state
            write_event(t)?;
            return Ok(());
        };

        let Some(last_time) = self.double_tap_state.get(&key.into()) else {
            // this key wasnt registered yet we dont care to check if double clicked
            self.double_tap_state.insert(key.into(), Instant::now());
            self.keys_state.insert(key.into(), (*key_event).into());
            write_event(t)?;
            return Ok(());
        };
        //this will never fail (i think lol). We always insert key state in the last let else
        let key_state = self.keys_state.get(&key.into()).unwrap();

        match (key_state, key_event) {
            (KeyState::Pressed, KeyEvent::Release) => {
                self.keys_state.insert(key.into(), KeyState::Released);
                write_event(t)?;
            }
            // dont do anythin cuz we just started holdin
            (KeyState::Held, KeyEvent::Release) => {}
            (KeyState::Held, KeyEvent::Press) => {
                self.keys_state.insert(key.into(), KeyState::Pressed);
                write_event(t)?;
            }
            (KeyState::Released, KeyEvent::Press) => {
                if last_time.elapsed().as_millis() < DOUBLE_TAP_TIME_MS {
                    self.keys_state.insert(key.into(), KeyState::Held);
                    write_event(t)?;
                } else {
                    self.keys_state.insert(key.into(), KeyState::Pressed);
                    self.double_tap_state.insert(key.into(), Instant::now());
                    write_event(t)?;
                }
            }
            (KeyState::Released, KeyEvent::Release) => {}
            (KeyState::Pressed, KeyEvent::Press) => {}
        }

        Ok(())
    }

    pub fn synchronize(&mut self) -> anyhow::Result<()> {
        let Ok(mut vjoy) = VJOY.lock() else {
            return Err(anyhow!("Failed to get vjoy"));
        };
        vjoy.update_device_state(&self.device)?;
        Ok(())
    }
}

impl Drop for Controller {
    fn drop(&mut self) {
        let Ok(mut device_id) = VJOY_ID.lock() else {
            return;
        };
        *device_id -= 1;
    }
}

impl From<KeyEvent> for ButtonState {
    fn from(value: KeyEvent) -> Self {
        match value {
            KeyEvent::Release => ButtonState::Released,
            KeyEvent::Press => ButtonState::Pressed,
        }
    }
}

enum Value {
    Axis(i32),
    Button(ButtonState),
}

impl From<KeyEvent> for Value {
    fn from(value: KeyEvent) -> Self {
        Self::Button(value.into())
    }
}

impl From<Key> for (u8, Value) {
    fn from(value: Key) -> Self {
        match value {
            Key::A(state) => (1, state.into()),
            Key::B(state) => (2, state.into()),
            Key::X(state) => (3, state.into()),
            Key::Y(state) => (4, state.into()),
            Key::LeftJoystickX(x) => (1, Value::Axis(map_vjoy(x))),
            Key::LeftJoystickY(y) => (2, Value::Axis(map_vjoy(-y))),
            Key::RightJoystickX(x) => (3, Value::Axis(map_vjoy(x))),
            Key::RightJoystickY(y) => (4, Value::Axis(map_vjoy(-y))),
            Key::BumperLeft(state) => (5, state.into()),
            Key::BumperRight(state) => (6, state.into()),
            Key::TriggerLeft(state) => (7, state.into()),
            Key::TriggerRight(state) => (8, state.into()),
            Key::Select(state) => (9, state.into()),
            Key::Start(state) => (10, state.into()),
            Key::DPadUp(state) => (13, state.into()),
            Key::DPadDown(state) => (14, state.into()),
            Key::DPadLeft(state) => (15, state.into()),
            Key::DPadRight(state) => (16, state.into()),
        }
    }
}

fn map_vjoy(value: f32) -> i32 {
    let normalized_value = (value + 1.0) / 2.0;
    (normalized_value * 32767.0 + 0.5) as i32
}
