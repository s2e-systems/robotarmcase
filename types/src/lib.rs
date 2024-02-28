use dust_dds::topic_definition::type_support::DdsType;

// ----------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, DdsType, Debug)]
pub struct Presence {
    pub present: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, DdsType, Debug)]
pub struct SensorState {
    pub is_on: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, DdsType, Debug)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Clone, Copy, Eq, PartialEq, DdsType, Debug)]
pub struct MotorSpeed {
    pub speed: i32,
}

#[derive(Clone, Copy, PartialEq, DdsType, Debug)]
pub struct DobotPose {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: f32,
}

#[derive(Clone, Copy, PartialEq, DdsType, Debug)]
pub struct Suction {
    pub is_on: bool
}
