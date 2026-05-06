use std::sync::{LazyLock, Mutex};
use std::thread;

use anyhow::anyhow;
use vigem_rust::{Client, TargetHandle, X360Button, X360Report, target::Xbox360};

use crate::input::{Key, KeyEvent};

static VIGEM: LazyLock<Result<Mutex<Client>, String>> = LazyLock::new(|| {
    Client::connect()
        .map(Mutex::new)
        .map_err(|err| err.to_string())
});
pub struct Controller {
    device: TargetHandle<Xbox360>,
    report: X360Report,
}

impl Controller {
    pub fn new(_device_name: &str) -> anyhow::Result<Self> {
        let vigem = VIGEM
            .as_ref()
            .map_err(|err| anyhow!("Failed to connect to vigem: {err}"))?;

        let Ok(vigem) = vigem.lock() else {
            return Err(anyhow!("Failed to lock vigem client"));
        };

        let device = vigem.new_x360_target().plugin()?;
        device.wait_for_ready()?;
        let notification_receiver = device.register_notification()?;
        thread::spawn(move || {
            // This loop will exit when the `x360` handle is dropped.
            while let Ok(Ok(notification)) = notification_receiver.recv() {
                println!("Received notification: {:?}", notification);
            }
        });

        Ok(Self {
            device,
            report: X360Report::default(),
        })
    }
    pub fn write_input(&mut self, key: Key) -> anyhow::Result<()> {
        match key {
            Key::LeftJoystickX(v) => self.report.thumb_lx = map_vigem(v),
            Key::LeftJoystickY(v) => self.report.thumb_ly = map_vigem(v),
            Key::RightJoystickX(v) => self.report.thumb_rx = map_vigem(v),
            Key::RightJoystickY(v) => self.report.thumb_ry = map_vigem(v),
            Key::ThumbRight(key_event) => self
                .report
                .buttons
                .set(X360Button::RIGHT_THUMB, key_event.into()),
            Key::ThumbLeft(key_event) => self
                .report
                .buttons
                .set(X360Button::LEFT_THUMB, key_event.into()),
            Key::DPadUp(key_event) => self
                .report
                .buttons
                .set(X360Button::DPAD_UP, key_event.into()),
            Key::DPadDown(key_event) => self
                .report
                .buttons
                .set(X360Button::DPAD_DOWN, key_event.into()),
            Key::DPadLeft(key_event) => self
                .report
                .buttons
                .set(X360Button::DPAD_LEFT, key_event.into()),
            Key::DPadRight(key_event) => self
                .report
                .buttons
                .set(X360Button::DPAD_RIGHT, key_event.into()),
            Key::A(key_event) => self.report.buttons.set(X360Button::A, key_event.into()),
            Key::B(key_event) => self.report.buttons.set(X360Button::B, key_event.into()),
            Key::X(key_event) => self.report.buttons.set(X360Button::X, key_event.into()),
            Key::Y(key_event) => self.report.buttons.set(X360Button::Y, key_event.into()),
            Key::Start(key_event) => self.report.buttons.set(X360Button::START, key_event.into()),
            Key::Select(key_event) => self.report.buttons.set(X360Button::BACK, key_event.into()),
            Key::TriggerLeft(key_event) => self.report.left_trigger = map_trigger(key_event),
            Key::BumperLeft(key_event) => self
                .report
                .buttons
                .set(X360Button::LEFT_SHOULDER, key_event.into()),
            Key::TriggerRight(key_event) => self.report.right_trigger = map_trigger(key_event),
            Key::BumperRight(key_event) => self
                .report
                .buttons
                .set(X360Button::RIGHT_SHOULDER, key_event.into()),
            Key::Mode(key_event) => self.report.buttons.set(X360Button::GUIDE, key_event.into()),
        }

        Ok(())
    }

    pub fn synchronize(&self) -> anyhow::Result<()> {
        self.device.update(&self.report)?;
        Ok(())
    }
}

fn map_vigem(value: f32) -> i16 {
    let clamped = value.clamp(-1.0, 1.0);
    (clamped * i16::MAX as f32).round() as i16
}

fn map_trigger(value: KeyEvent) -> u8 {
    if bool::from(value) { u8::MAX } else { 0 }
}
