use dust_dds::{
    publication::data_writer::DataWriter,
};
use types::{DobotArmMovement, DobotPose, MotorSpeed, Suction};

use crate::reader::Reader;

// ----------------------------------------------------------------------------

const CONVEYOR_BELT_SPEED: MotorSpeed = MotorSpeed { speed: 7500 };

const TOLERANCE: f32 = 0.5;

const INITIAL_POSITION: DobotPose = DobotPose {
    x: 165.0,
    y: -5.0,
    z: 30.0,
    r: 0.0,
};
const ABOVE_BLOCK_POSITION: DobotPose = DobotPose {
    x: 248.0,
    y: -113.0,
    z: 11.0,
    r: 0.0,
};
const BLOCK_PICKUP_POSITION: DobotPose = DobotPose {
    x: 248.0,
    y: -113.0,
    z: 0.0,
    r: 0.0,
};
const COLOR_SENSOR_POSITION: DobotPose = DobotPose {
    x: 166.0,
    y: 54.0,
    z: 26.0,
    r: -8.0,
};
const BLOCK_DISPOSE_RED: DobotPose = DobotPose {
    x: 150.0,
    y: 162.0,
    z: 30.0,
    r: -15.0,
};
const BLOCK_DISPOSE_GREEN: DobotPose = DobotPose {
    x: 103.0,
    y: 173.0,
    z: 30.0,
    r: -15.0,
};
const BLOCK_DISPOSE_BLUE: DobotPose = DobotPose {
    x: 63.0,
    y: 175.0,
    z: 30.0,
    r: -15.0,
};
const BLOCK_DISPOSE_MIXED: DobotPose = DobotPose {
    x: 20.0,
    y: 177.0,
    z: 33.0,
    r: -10.0,
};

pub enum State {
    Initial,
    GetReady,
    WaitForBlock,
    PickUpBlock,
    CheckColor,
    MoveToRed,
    MoveToGreen,
    MoveToBlue,
    MoveToMixed,
    DropBlock,
}

pub struct Controller {
    conveyor_belt_writer: DataWriter<MotorSpeed>,
    pose_writer: DataWriter<DobotArmMovement>,
    suction_writer: DataWriter<Suction>,
    destination: DobotPose,
    pub state: State,
}

#[rustfmt::skip]
fn distance(p1: DobotPose, p2: DobotPose) -> f32 {
    ( (p1.x - p2.x).powf(2.0)
    + (p1.y - p2.y).powf(2.0)
    + (p1.z - p2.z).powf(2.0)
    ).sqrt()
}

impl Controller {
    pub fn new(
        conveyor_belt_writer: DataWriter<MotorSpeed>,
        pose_writer: DataWriter<DobotArmMovement>,
        suction_writer: DataWriter<Suction>,
    ) -> Self {
        let mut controller = Self {
            conveyor_belt_writer,
            pose_writer,
            suction_writer,
            destination: INITIAL_POSITION,
            state: State::Initial,
        };
        controller.initial();
        controller
    }

    pub fn is_arrived<const R: u64>(&self, dobot_pose: &Reader<DobotPose, R>) -> bool {
        dobot_pose
            .value()
            .map(|current_pose| distance(self.destination, current_pose) < TOLERANCE)
            .unwrap_or(false)
    }

    pub fn initial(&mut self) {
        self.state = State::Initial;
        self.destination = INITIAL_POSITION;
        self.conveyor_belt_writer
            .write(&MotorSpeed { speed: 0 }, None)
            .unwrap();
        self.suction_writer.write(&Suction::Off, None).unwrap();
        self.pose_writer
            .write(&DobotArmMovement::Joint(self.destination), None)
            .unwrap();
    }

    pub fn get_ready(&mut self) {
        self.state = State::GetReady;
        self.destination = ABOVE_BLOCK_POSITION;
        self.pose_writer
            .write(&DobotArmMovement::Joint(self.destination), None)
            .unwrap();
    }

    pub fn wait_for_block(&mut self) {
        self.state = State::WaitForBlock;
        self.destination = ABOVE_BLOCK_POSITION;
        self.conveyor_belt_writer
            .write(&CONVEYOR_BELT_SPEED, None)
            .unwrap();
        self.pose_writer
            .write(&DobotArmMovement::Joint(self.destination), None)
            .unwrap();
    }

    pub fn pick_up_block(&mut self) {
        self.state = State::PickUpBlock;
        self.destination = BLOCK_PICKUP_POSITION;
        self.conveyor_belt_writer
            .write(&MotorSpeed { speed: 0 }, None)
            .unwrap();
        self.suction_writer.write(&Suction::On, None).unwrap();
        self.pose_writer
            .write(&DobotArmMovement::Joint(self.destination), None)
            .unwrap();
    }

    fn move_block_to(&mut self, state: State, destination: DobotPose) {
        self.state = state;
        self.destination = destination;

        self.conveyor_belt_writer
            .write(&MotorSpeed { speed: 0 }, None)
            .unwrap();
        self.pose_writer
            .write(&DobotArmMovement::Joint(self.destination), None)
            .unwrap();
    }

    pub fn check_color(&mut self) {
        self.move_block_to(State::CheckColor, COLOR_SENSOR_POSITION);
    }

    pub fn move_to_red(&mut self) {
        self.move_block_to(State::MoveToRed, BLOCK_DISPOSE_RED);
    }

    pub fn move_to_green(&mut self) {
        self.move_block_to(State::MoveToGreen, BLOCK_DISPOSE_GREEN);
    }

    pub fn move_to_blue(&mut self) {
        self.move_block_to(State::MoveToBlue, BLOCK_DISPOSE_BLUE);
    }

    pub fn move_to_mixed(&mut self) {
        self.move_block_to(State::MoveToMixed, BLOCK_DISPOSE_MIXED);
    }

    pub fn drop_block(&mut self) {
        self.state = State::DropBlock;
        self.suction_writer.write(&Suction::Off, None).unwrap();
    }
}