use std::sync::{LazyLock, Mutex};

use log::info;
use vjoy::{ButtonState, Device, VJoy};

use crate::{keys::Keys, message::State};
use anyhow::anyhow;

pub struct Controller {
    device: Device,
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
        Ok(Self { device })
    }
    pub fn send_key(&mut self, key: Keys) -> anyhow::Result<()> {
        let t: (u8, Value) = key.into();

        match t {
            (axis, Value::Axis(v)) => self.device.set_axis(axis as u32, v),
            (key, Value::Button(state)) => self.device.set_button(key, state),
        }?;
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

impl From<State> for ButtonState {
    fn from(value: State) -> Self {
        match value {
            State::Release => ButtonState::Released,
            State::Press => ButtonState::Pressed,
        }
    }
}

enum Value {
    Axis(i32),
    Button(ButtonState),
}

impl From<State> for Value {
    fn from(value: State) -> Self {
        Self::Button(value.into())
    }
}

impl From<Keys> for (u8, Value) {
    fn from(value: Keys) -> Self {
        match value {
            Keys::A(state) => (1, state.into()),
            Keys::B(state) => (2, state.into()),
            Keys::X(state) => (3, state.into()),
            Keys::Y(state) => (4, state.into()),
            Keys::LeftJoystickX(x) => (1, Value::Axis(map_vjoy(x))),
            Keys::LeftJoystickY(y) => (2, Value::Axis(map_vjoy(-y))),
            Keys::RightJoystickX(x) => (3, Value::Axis(map_vjoy(x))),
            Keys::RightJoystickY(y) => (4, Value::Axis(map_vjoy(-y))),
            Keys::BumperLeft(state) => (5, state.into()),
            Keys::BumperRight(state) => (6, state.into()),
            Keys::TriggerLeft(state) => (7, state.into()),
            Keys::TriggerRight(state) => (8, state.into()),
            Keys::Select(state) => (9, state.into()),
            Keys::Start(state) => (10, state.into()),
            Keys::DPadUp(state) => (13, state.into()),
            Keys::DPadDown(state) => (14, state.into()),
            Keys::DPadLeft(state) => (15, state.into()),
            Keys::DPadRight(state) => (16, state.into()),
        }
    }
}

fn map_vjoy(value: f32) -> i32 {
    let normalized_value = (value + 1.0) / 2.0;
    (normalized_value * 32767.0 + 0.5) as i32
}
