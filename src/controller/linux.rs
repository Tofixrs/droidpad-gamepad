use std::collections::HashMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use evdev_rs::InputEvent;
use evdev_rs::TimeVal;
use evdev_rs::UInputDevice;
use evdev_rs::enums::EV_ABS;
use evdev_rs::enums::EV_KEY;
use evdev_rs::enums::EV_SYN;
use evdev_rs::enums::EventCode;

use anyhow::anyhow;

use crate::controller::{DOUBLE_TAP_TIME_MS, KeyState};
use crate::keys::Key;
use crate::message::KeyEvent;

const UINPUT_AXIS_MIN: i32 = -32768;
const UINPUT_AXIS_MAX: i32 = 32767;

pub struct Controller {
    device: UInputDevice,
    keys_state: HashMap<u8, KeyState>,
    double_tap_state: HashMap<u8, Instant>,
}

impl Controller {
    pub fn new(device_name: &str) -> anyhow::Result<Self> {
        use evdev_rs::{
            AbsInfo, DeviceWrapper, UninitDevice,
            enums::{BusType, EV_ABS, EV_KEY, EV_SYN, EventCode},
        };

        let u = UninitDevice::new().ok_or(anyhow!("Failed to create UninitDevice"))?;
        u.set_name(device_name);
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

        Ok(Self {
            device: UInputDevice::create_from_device(&u)?,
            keys_state: HashMap::default(),
            double_tap_state: HashMap::default(),
        })
    }
    pub fn handle_key(&mut self, key: Key) -> anyhow::Result<()> {
        let Some(key_event) = key.key_event() else {
            //we ignore joysticks; they dont have btn state
            self.device.write_event(&key.into())?;
            return Ok(());
        };

        let Some(last_time) = self.double_tap_state.get(&key.into()) else {
            // this key wasnt registered yet we dont care to check if double clicked
            self.double_tap_state.insert(key.into(), Instant::now());
            self.keys_state.insert(key.into(), (*key_event).into());
            self.device.write_event(&key.into())?;
            return Ok(());
        };
        //this will never fail (i think lol). We always insert key state in the last let else
        let key_state = self.keys_state.get(&key.into()).unwrap();

        match (key_state, key_event) {
            (KeyState::Pressed, KeyEvent::Release) => {
                self.keys_state.insert(key.into(), KeyState::Released);
                self.device.write_event(&key.into())?;
            }
            // dont do anythin cuz we just started holdin
            (KeyState::Held, KeyEvent::Release) => {}
            (KeyState::Held, KeyEvent::Press) => {
                self.keys_state.insert(key.into(), KeyState::Pressed);
                self.device.write_event(&key.into())?;
            }
            (KeyState::Released, KeyEvent::Press) => {
                if last_time.elapsed().as_millis() < DOUBLE_TAP_TIME_MS {
                    self.keys_state.insert(key.into(), KeyState::Held);
                    self.device.write_event(&key.into())?;
                } else {
                    self.keys_state.insert(key.into(), KeyState::Pressed);
                    self.double_tap_state.insert(key.into(), Instant::now());
                    self.device.write_event(&key.into())?;
                }
            }
            (KeyState::Released, KeyEvent::Release) => {}
            (KeyState::Pressed, KeyEvent::Press) => {}
        }

        Ok(())
    }
    pub fn synchronize(&self) -> anyhow::Result<()> {
        self.device.write_event(&InputEvent::new(
            &timeval_now(),
            &EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            0,
        ))?;

        Ok(())
    }
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

fn map_float_to_axis_value(f: f32) -> i32 {
    let scaled_value =
        ((f + 1.0) / 2.0) * (UINPUT_AXIS_MAX - UINPUT_AXIS_MIN) as f32 + UINPUT_AXIS_MIN as f32;
    scaled_value.round() as i32
}

impl From<Key> for InputEvent {
    fn from(val: Key) -> Self {
        let (ev_code, val) = match val {
            Key::LeftJoystickX(v) => (EventCode::EV_ABS(EV_ABS::ABS_X), map_float_to_axis_value(v)),
            Key::LeftJoystickY(v) => (
                EventCode::EV_ABS(EV_ABS::ABS_Y),
                -map_float_to_axis_value(v),
            ),
            Key::RightJoystickX(v) => (
                EventCode::EV_ABS(EV_ABS::ABS_RX),
                map_float_to_axis_value(v),
            ),
            Key::RightJoystickY(v) => (
                EventCode::EV_ABS(EV_ABS::ABS_RY),
                -map_float_to_axis_value(v),
            ),
            Key::DPadUp(state) => (EventCode::EV_KEY(EV_KEY::BTN_DPAD_UP), state as i32),
            Key::DPadDown(state) => (EventCode::EV_KEY(EV_KEY::BTN_DPAD_DOWN), state as i32),
            Key::DPadLeft(state) => (EventCode::EV_KEY(EV_KEY::BTN_DPAD_LEFT), state as i32),
            Key::DPadRight(state) => (EventCode::EV_KEY(EV_KEY::BTN_DPAD_RIGHT), state as i32),

            Key::A(state) => (EventCode::EV_KEY(EV_KEY::BTN_SOUTH), state as i32),
            Key::B(state) => (EventCode::EV_KEY(EV_KEY::BTN_EAST), state as i32),
            Key::X(state) => (EventCode::EV_KEY(EV_KEY::BTN_WEST), state as i32),
            Key::Y(state) => (EventCode::EV_KEY(EV_KEY::BTN_NORTH), state as i32),
            Key::Start(state) => (EventCode::EV_KEY(EV_KEY::BTN_START), state as i32),
            Key::Select(state) => (EventCode::EV_KEY(EV_KEY::BTN_SELECT), state as i32),
            Key::TriggerLeft(state) => (EventCode::EV_KEY(EV_KEY::BTN_TL2), state as i32),
            Key::BumperLeft(state) => (EventCode::EV_KEY(EV_KEY::BTN_TL), state as i32),
            Key::TriggerRight(state) => (EventCode::EV_KEY(EV_KEY::BTN_TR2), state as i32),
            Key::BumperRight(state) => (EventCode::EV_KEY(EV_KEY::BTN_TR), state as i32),
        };

        InputEvent::new(&timeval_now(), &ev_code, val)
    }
}
