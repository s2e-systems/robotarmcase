use std::fmt::Display;

use dust_dds::infrastructure::type_support::DdsType;

#[derive(Copy, Clone, PartialEq, Eq, DdsType, Debug)]
pub enum Presence {
    Present,
    NotPresent,
}

#[derive(Copy, Clone, PartialEq, Eq, DdsType, Debug)]
pub enum SensorState {
    Available,
    NotAvailable,
}

#[derive(Copy, Clone, PartialEq, Eq, DdsType, Debug)]
pub enum Color {
    Red,
    Green,
    Blue,
    Undefined,
}

#[derive(Clone, Copy, Eq, PartialEq, DdsType, Debug)]
pub struct ConveyorBeltSpeed {
    pub speed: i32,
}

#[derive(Clone, Copy, PartialEq, DdsType, Debug)]
pub struct DobotPose {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: f32,
}
impl Display for DobotPose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[x:{:.2}, y:{:.2}, z:{:.2}, r:{:.2}]",
            self.x, self.y, self.z, self.r
        )
    }
}

#[derive(Clone, Copy, PartialEq, DdsType, Debug)]
pub enum Suction {
    On,
    Off,
}
impl Display for Suction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let out = match self {
            Suction::On => "On",
            Suction::Off => "Off",
        };
        write!(f, "{out}")
    }
}
