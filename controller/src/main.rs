use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{listeners::NoOpListener, qos::QosKind, status::NO_STATUS},
};

use std::io::{stdout, Write};
use types::{SensorState, Color, DobotArmMovement, DobotPose, MotorSpeed, PresenceSensor, Suction};

// mod reader;
// use reader::{Reader, Sensor};

// mod controller;
// use controller::{Controller, State};

// ----------------------------------------------------------------------------

const LOOP_PERIOD_MS: u64 = 50;

fn show_dobot_pose(pose: &Option<DobotPose>) -> String {
    match pose {
        None => "unknown".to_string(),
        Some(pose) => format!(
            "{{x: {:.2}, y: {:.2}, z: {:.2}, r: {:.2}}}",
            pose.x, pose.y, pose.z, pose.r
        ),
    }
}

fn main() {
    let domain_id = 0;

    let participant_factory = DomainParticipantFactory::get_instance();
    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NoOpListener::new(), NO_STATUS)
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NoOpListener::new(), NO_STATUS)
        .unwrap();

    // let mut presence_sensor = {
        let topic_presence_availability = participant
            .create_topic::<SensorState>(
                "SensorState",
                "PresenceSensorAvailability",
                QosKind::Default,
                NoOpListener::new(),
                NO_STATUS,
            )
            .unwrap();
        let topic_presence = participant
            .create_topic::<PresenceSensor>("Presence", "PresenceSensor", QosKind::Default, NoOpListener::new(), NO_STATUS)
            .unwrap();
    //     Sensor::<Presence>::new(&topic_presence_availability, &topic_presence, &subscriber).unwrap()
    // };
    // let mut color_sensor = {
    //     let topic_color_availability = participant
    //         .create_topic(
    //             "ColorSensorAvailability",
    //             QosKind::Default,
    //             None,
    //             NO_STATUS,
    //         )
    //         .unwrap();
    //     let topic_color = participant
    //         .create_topic("ColorSensor", QosKind::Default, None, NO_STATUS)
    //         .unwrap();
    //     Sensor::<Color>::new(&topic_color_availability, &topic_color, &subscriber).unwrap()
    // };
    // let mut dobot_pose = {
    //     let topic_current_pose = participant
    //         .create_topic("CurrentDobotPose", QosKind::Default, None, NO_STATUS)
    //         .unwrap();
    //     Reader::<DobotPose>::new(&topic_current_pose, &subscriber).unwrap()
    // };
    // let mut suction = {
    //     let topic_suction = participant
    //         .create_topic("CurrentSuctionCupState", QosKind::Default, None, NO_STATUS)
    //         .unwrap();
    //     Reader::<Suction>::new(&topic_suction, &subscriber).unwrap()
    // };

    // let publisher = participant
    //     .create_publisher(QosKind::Default, None, NO_STATUS)
    //     .unwrap();

    // let mut controller = {
    //     let topic_conveyor_belt_speed = participant
    //         .create_topic::<MotorSpeed>("ConveyorBeltSpeed", QosKind::Default, None, NO_STATUS)
    //         .unwrap();
    //     let topic_pose = participant
    //         .create_topic::<DobotArmMovement>("DobotArmMovement", QosKind::Default, None, NO_STATUS)
    //         .unwrap();
    //     let topic_suction = participant
    //         .create_topic::<Suction>("SuctionCup", QosKind::Default, None, NO_STATUS)
    //         .unwrap();

    //     Controller::new(
    //         publisher
    //             .create_datawriter(
    //                 &topic_conveyor_belt_speed,
    //                 QosKind::Default,
    //                 None,
    //                 NO_STATUS,
    //             )
    //             .unwrap(),
    //         publisher
    //             .create_datawriter(&topic_pose, QosKind::Default, None, NO_STATUS)
    //             .unwrap(),
    //         publisher
    //             .create_datawriter(&topic_suction, QosKind::Default, None, NO_STATUS)
    //             .unwrap(),
    //     )
    // };

    // loop {
    //     presence_sensor.update();
    //     color_sensor.update();
    //     dobot_pose.update();
    //     suction.update();

    //     match controller.state {
    //         State::Initial => if presence_sensor.value().is_some() {
    //             controller.get_ready();
    //         },

    //         State::GetReady if controller.is_arrived(&dobot_pose) => {
    //             controller.wait_for_block();
    //         }

    //         State::WaitForBlock => match presence_sensor.value() {
    //             Some(Presence::Present) => controller.pick_up_block(),
    //             Some(Presence::NotPresent) => (),
    //             None => controller.initial(),
    //         },

    //         State::PickUpBlock if controller.is_arrived(&dobot_pose) => {
    //             if suction.value() == &Some(Suction::On) {
    //                 match color_sensor.value() {
    //                     Some(_) => controller.check_color(),
    //                     None => controller.move_to_mixed(),
    //                 }
    //             }
    //         }

    //         State::CheckColor if controller.is_arrived(&dobot_pose) => {
    //             std::thread::sleep(std::time::Duration::from_millis(500));
    //             color_sensor.update();
    //             match color_sensor.value() {
    //                 Some(Color::Red) => controller.move_to_red(),
    //                 Some(Color::Green) => controller.move_to_green(),
    //                 Some(Color::Blue) => controller.move_to_blue(),
    //                 _ => controller.move_to_mixed(),
    //             }
    //         }

    //         State::MoveToRed | State::MoveToGreen | State::MoveToBlue | State::MoveToMixed => {
    //             if controller.is_arrived(&dobot_pose) {
    //                 controller.drop_block();
    //             }
    //         }

    //         State::DropBlock => {
    //             if suction.value() == &Some(Suction::Off) {
    //                 controller.get_ready();
    //             }
    //         }

    //         _ => (),
    //     };

    //     print!("PRESENCE: {:<11}", presence_sensor);
    //     print!("  ");
    //     print!("COLOR: {:<13}", color_sensor);
    //     print!("  ");
    //     print!("DOBOT POSE: {:<50}", show_dobot_pose(dobot_pose.value()));
    //     print!("\r");
    //     stdout().flush().unwrap();

    //     std::thread::sleep(std::time::Duration::from_millis(LOOP_PERIOD_MS));
    // }
}
