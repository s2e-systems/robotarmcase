mod controller;

use controller::{Controller, State};
use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listeners::NoOpListener,
        qos::QosKind,
        status::{StatusKind, NO_STATUS},
        time::Duration,
        wait_set::{Condition, WaitSet},
    },
    subscription::{
        data_reader::DataReader,
        sample_info::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE},
    },
};
use std::{
    io::{stdout, Write},
    time::Instant,
};
use types::{Color, DobotPose, MotorSpeed, PresenceSensor, SensorState, Suction};

use crate::controller::CONVEYOR_BELT_SPEED;

const LOOP_PERIOD: std::time::Duration = std::time::Duration::from_millis(5);

fn show_dobot_pose(pose: &Option<DobotPose>) -> String {
    match pose {
        None => "unknown".to_string(),
        Some(pose) => format!(
            "{{x: {:.2}, y: {:.2}, z: {:.2}, r: {:}}}",
            pose.x, pose.y, pose.z, pose.r
        ),
    }
}

fn is_sensor_available(reader: &DataReader<SensorState>) -> bool {
    if let Ok(sample_list) = reader.read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE) {
        if let Some(sample) = sample_list.first() {
            sample.data().is_ok_and(|d: SensorState| d.is_on)
        } else {
            false
        }
    } else {
        false
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

    let presence_sensor_availability_reader = subscriber
        .create_datareader(
            &topic_presence_availability,
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();
    let presence_reader = subscriber
        .create_datareader(
            &topic_presence,
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();

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

    let color_reader = subscriber
        .create_datareader(
            &topic_color,
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();

    let color_sensor_availability_reader = subscriber
        .create_datareader(
            &topic_color_availability,
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();

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

    let mut controller = Controller::new(
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
    );

    let writer_cond = controller
        .conveyor_belt_writer
        .get_statuscondition()
        .unwrap();
    writer_cond
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(writer_cond))
        .unwrap();

    wait_set.wait(Duration::new(60, 0)).unwrap();
    println!("conveyor belt matched");

    controller.initial();

    loop {
        let start = Instant::now();

        let dobot_pose = if let Ok(sample_list) =
            dobot_pose_reader.read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        {
            if let Some(sample) = sample_list.first() {
                sample.data().ok()
            } else {
                None
            }
        } else {
            None
        };

        if !is_sensor_available(&presence_sensor_availability_reader) {
            controller
                .conveyor_belt_writer
                .write(&MotorSpeed { speed: 0 }, None)
                .unwrap();
        }

        match controller.state {
            State::Initial => {
                if is_sensor_available(&presence_sensor_availability_reader) {
                    controller.get_ready();
                }
            }

            State::GetReady if controller.is_arrived(&dobot_pose) => {
                controller.wait_for_block();
            }

            State::WaitForBlock => {
                if is_sensor_available(&presence_sensor_availability_reader) {
                    controller
                        .conveyor_belt_writer
                        .write(&CONVEYOR_BELT_SPEED, None)
                        .unwrap();
                }

                if let Ok(sample_list) =
                    presence_reader.read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
                {
                    if let Some(sample) = sample_list.first() {
                        if let Ok(PresenceSensor { present: true }) = sample.data() {
                            controller.pick_up_block();
                        }
                    }
                }
            }

            State::PickUpBlock if controller.is_arrived(&dobot_pose) => {
                controller.lift_up_block();
            }

            State::LiftUpBlock if controller.is_arrived(&dobot_pose) => {
                if let Ok(sample_list) =
                    suction_reader.read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
                {
                    if let Some(sample) = sample_list.first() {
                        if let Ok(Suction { is_on: true }) = sample.data() {
                            match is_sensor_available(&color_sensor_availability_reader) {
                                true => controller.check_color(),
                                false => controller.move_to_mixed(),
                            }
                        }
                    }
                }
            }

            State::CheckColor if controller.is_arrived(&dobot_pose) => {
                if let Ok(sample_list) =
                    color_reader.read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
                {
                    if let Some(sample) = sample_list.first() {
                        if let Ok(color) = sample.data() {
                            controller.color = color;
                        }
                    }
                };
                if controller.time.elapsed() > std::time::Duration::from_millis(1000) {
                    controller.lift_up_from_color();
                }
            }

            State::LiftUpFromColor if controller.is_arrived(&dobot_pose) => {
                match controller.color {
                    Color { red: 255, .. } => controller.move_to_red(),
                    Color { green: 255, .. } => controller.move_to_green(),
                    Color { blue: 255, .. } => controller.move_to_blue(),
                    _ => controller.move_to_mixed(),
                }
            }

            State::MoveToRed | State::MoveToGreen | State::MoveToBlue | State::MoveToMixed => {
                if controller.is_arrived(&dobot_pose) {
                    controller.drop_block();
                }
            }

            State::DropBlock => {
                if let Ok(sample_list) =
                    suction_reader.read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
                {
                    if let Some(sample) = sample_list.first() {
                        if let Ok(Suction { is_on: false }) = sample.data() {
                            controller.get_ready();
                        }
                    }
                }
            }

            _ => (),
        };

        print!("STATE: {:<15?}", controller.state);
        print!("  DOBOT POSE: {:<50}", show_dobot_pose(&dobot_pose));

        if let Some(time_remaining) = LOOP_PERIOD.checked_sub(start.elapsed()) {
            std::thread::sleep(time_remaining);
            print!("  REMAINING TIME: {:?}", time_remaining)
        } else {
            print!("  REMAINING TIME: CPU overload")
        }

        print!("\r");
        stdout().flush().unwrap();
    }
}
