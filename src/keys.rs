use crate::message::State;

pub enum Keys {
    LeftJoystickX(f32),
    LeftJoystickY(f32),
    RightJoystickX(f32),
    RightJoystickY(f32),
    DPadUp(State),
    DPadDown(State),
    DPadLeft(State),
    DPadRight(State),
    A(State),
    B(State),
    X(State),
    Y(State),
    Start(State),
    Select(State),
    TriggerLeft(State),
    BumperLeft(State),
    TriggerRight(State),
    BumperRight(State),
}
