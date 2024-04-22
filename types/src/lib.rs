use dust_dds::topic_definition::type_support::DdsType;

// ----------------------------------------------------------------------------

#[derive(Copy, Clone, PartialEq, Eq, DdsType, Debug)]
pub enum Presence {
    Present,
    NotPresent
}

#[derive(Copy, Clone, PartialEq, Eq, DdsType, Debug)]
pub enum SensorState {
    Available,
    NotAvailable
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

#[derive(Clone, Copy, PartialEq, DdsType, Debug)]
pub enum Suction {
    On,
    Off
}
