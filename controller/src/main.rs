mod controller;
mod reader;

use controller::{Controller, State};
use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{listeners::NoOpListener, qos::QosKind, status::NO_STATUS},
    subscription::sample_info::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE},
};
use reader::Sensor;
use std::io::{stdout, Write};
use types::{Color, DobotPose, MotorSpeed, PresenceSensor, SensorState, Suction};

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

    let mut presence_sensor = {
        let topic_presence_availability = participant
            .create_topic::<SensorState>(
                "PresenceSensorAvailability",
                "SensorState",
                QosKind::Default,
                NoOpListener::new(),
                NO_STATUS,
            )
            .unwrap();
        let topic_presence = participant
            .create_topic::<PresenceSensor>(
                "Presence",
                "PresenceSensor",
                QosKind::Default,
                NoOpListener::new(),
                NO_STATUS,
            )
            .unwrap();
        Sensor::<PresenceSensor>::new(&topic_presence_availability, &topic_presence, &subscriber)
            .unwrap()
    };
    let mut color_sensor = {
        let topic_color_availability = participant
            .create_topic::<SensorState>(
                "ColorSensorAvailability",
                "SensorState",
                QosKind::Default,
                NoOpListener::new(),
                NO_STATUS,
            )
            .unwrap();
        let topic_color = participant
            .create_topic::<Color>(
                "ColorSensor",
                "Color",
                QosKind::Default,
                NoOpListener::new(),
                NO_STATUS,
            )
            .unwrap();
        Sensor::<Color>::new(&topic_color_availability, &topic_color, &subscriber).unwrap()
    };

    let topic_current_pose = participant
        .create_topic::<DobotPose>(
            "CurrentDobotPose",
            "DobotPose",
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();
    let dobot_pose_reader = subscriber
        .create_datareader(
            &topic_current_pose,
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();
    let topic_suction = participant
        .create_topic::<Suction>(
            "CurrentSuctionCupState",
            "Suction",
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();
    let suction_reader = subscriber
        .create_datareader(
            &topic_suction,
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NoOpListener::new(), NO_STATUS)
        .unwrap();

    let mut controller = {
        let topic_conveyor_belt_speed = participant
            .create_topic::<MotorSpeed>(
                "ConveyorBeltSpeed",
                "MotorSpeed",
                QosKind::Default,
                NoOpListener::new(),
                NO_STATUS,
            )
            .unwrap();
        let topic_pose = participant
            .create_topic::<DobotPose>(
                "DobotArmMovement",
                "DobotPose",
                QosKind::Default,
                NoOpListener::new(),
                NO_STATUS,
            )
            .unwrap();
        let topic_suction = participant
            .create_topic::<Suction>(
                "SuctionCup",
                "Suction",
                QosKind::Default,
                NoOpListener::new(),
                NO_STATUS,
            )
            .unwrap();

        Controller::new(
            publisher
                .create_datawriter(
                    &topic_conveyor_belt_speed,
                    QosKind::Default,
                    NoOpListener::new(),
                    NO_STATUS,
                )
                .unwrap(),
            publisher
                .create_datawriter(
                    &topic_pose,
                    QosKind::Default,
                    NoOpListener::new(),
                    NO_STATUS,
                )
                .unwrap(),
            publisher
                .create_datawriter(
                    &topic_suction,
                    QosKind::Default,
                    NoOpListener::new(),
                    NO_STATUS,
                )
                .unwrap(),
        )
    };

    loop {
        presence_sensor.update();
        color_sensor.update();

        let dobot_pose = if let Ok(sample_list) =
            dobot_pose_reader.take(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        {
            if let Some(sample) = sample_list.first() {
                sample.data().ok()
            } else {
                None
            }
        } else {
            None
        };

        let suction = if let Ok(sample_list) =
            suction_reader.take(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        {
            if let Some(sample) = sample_list.first() {
                sample.data().ok()
            } else {
                None
            }
        } else {
            None
        };

        match controller.state {
            State::Initial => {
                if presence_sensor.value().is_some() {
                    controller.get_ready();
                }
            }

            State::GetReady if controller.is_arrived(&dobot_pose) => {
                controller.wait_for_block();
            }

            State::WaitForBlock => match presence_sensor.value() {
                Some(PresenceSensor { present: true }) => controller.pick_up_block(),
                Some(PresenceSensor { present: false }) => (),
                None => controller.initial(),
            },

            State::PickUpBlock if controller.is_arrived(&dobot_pose) => {
                if suction == Some(Suction { is_on: true }) {
                    match color_sensor.value() {
                        Some(_) => controller.check_color(),
                        None => controller.move_to_mixed(),
                    }
                }
            }

            State::CheckColor if controller.is_arrived(&dobot_pose) => {
                std::thread::sleep(std::time::Duration::from_millis(500));
                color_sensor.update();
                match color_sensor.value() {
                    Some(Color { red: 255, .. }) => controller.move_to_red(),
                    Some(Color { green: 255, .. }) => controller.move_to_green(),
                    Some(Color { blue: 255, .. }) => controller.move_to_blue(),
                    _ => controller.move_to_mixed(),
                }
            }

            State::MoveToRed | State::MoveToGreen | State::MoveToBlue | State::MoveToMixed => {
                if controller.is_arrived(&dobot_pose) {
                    controller.drop_block();
                }
            }

            State::DropBlock => {
                if suction == Some(Suction { is_on: false }) {
                    controller.get_ready();
                }
            }

            _ => (),
        };

        print!("PRESENCE: {:<11}", presence_sensor);
        print!("  ");
        print!("COLOR: {:<13}", color_sensor);
        print!("  ");
        print!("DOBOT POSE: {:<50}", show_dobot_pose(&dobot_pose));
        print!("\r");
        stdout().flush().unwrap();

        std::thread::sleep(std::time::Duration::from_millis(LOOP_PERIOD_MS));
    }
}
