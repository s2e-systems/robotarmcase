mod controller;

use crate::controller::CONVEYOR_BELT_SPEED;
use controller::{Controller, State};
use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        qos::{DataWriterQos, QosKind},
        qos_policy::{ReliabilityQosPolicy, ReliabilityQosPolicyKind},
        sample_info::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE},
        status::NO_STATUS,
        type_support::TypeSupport,
    },
    listener::NO_LISTENER,
    subscription::data_reader::DataReader,
};
use std::{
    io::{Write, stdout},
    time::Instant,
};
use types::{Color, ConveyorBeltSpeed, DobotPose, Presence, SensorState, Suction};

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
            sample.data.is_some_and(|d: SensorState| match d {
                SensorState::Available => true,
                SensorState::NotAvailable => false,
            })
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
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic_presence_availability = participant
        .create_topic::<SensorState>(
            "PresenceSensorAvailability",
            SensorState::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let topic_presence = participant
        .create_topic::<Presence>(
            "Presence",
            Presence::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let presence_sensor_availability_reader = subscriber
        .create_datareader(
            &topic_presence_availability,
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let presence_reader = subscriber
        .create_datareader(&topic_presence, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic_color_availability = participant
        .create_topic::<SensorState>(
            "ColorSensorAvailability",
            SensorState::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let topic_color = participant
        .create_topic::<Color>(
            "ColorSensor",
            Color::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let color_reader = subscriber
        .create_datareader(&topic_color, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let color_sensor_availability_reader = subscriber
        .create_datareader(
            &topic_color_availability,
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let topic_current_pose = participant
        .create_topic::<DobotPose>(
            "CurrentDobotPose",
            DobotPose::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let dobot_pose_reader = subscriber
        .create_datareader(
            &topic_current_pose,
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let topic_suction = participant
        .create_topic::<Suction>(
            "CurrentSuctionCupState",
            Suction::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let suction_reader = subscriber
        .create_datareader(&topic_suction, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic_conveyor_belt_speed = participant
        .create_topic::<ConveyorBeltSpeed>(
            "ConveyorBeltSpeed",
            ConveyorBeltSpeed::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let topic_pose = participant
        .create_topic::<DobotPose>(
            "DobotArmMovement",
            DobotPose::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let topic_suction = participant
        .create_topic::<Suction>(
            "SuctionCup",
            Suction::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let mut controller = Controller::new(
        publisher
            .create_datawriter(
                &topic_conveyor_belt_speed,
                QosKind::Specific(DataWriterQos {
                    reliability: ReliabilityQosPolicy {
                        kind: ReliabilityQosPolicyKind::Reliable,
                        max_blocking_time: dust_dds::infrastructure::time::DurationKind::Infinite,
                    },
                    ..Default::default()
                }),
                NO_LISTENER,
                NO_STATUS,
            )
            .unwrap(),
        publisher
            .create_datawriter(&topic_pose, QosKind::Default, NO_LISTENER, NO_STATUS)
            .unwrap(),
        publisher
            .create_datawriter(&topic_suction, QosKind::Default, NO_LISTENER, NO_STATUS)
            .unwrap(),
    );

    controller.initial();

    loop {
        let start = Instant::now();

        let dobot_pose = dobot_pose_reader
            .read_next_sample()
            .ok()
            .and_then(|sample| sample.data);

        if !is_sensor_available(&presence_sensor_availability_reader) {
            controller
                .conveyor_belt_writer
                .write(ConveyorBeltSpeed { speed: 0 }, None)
                .ok();
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
                        .write(CONVEYOR_BELT_SPEED, None)
                        .unwrap();
                }

                if let Ok(sample_list) =
                    presence_reader.read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
                {
                    if let Some(sample) = sample_list.first() {
                        if let Some(Presence::Present) = sample.data {
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
                        if let Some(Suction::On) = sample.data {
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
                        if let Some(color) = sample.data {
                            let color_str = match controller.color {
                                Color::Red => "red",
                                Color::Green => "green",
                                Color::Blue => "blue",
                                Color::Undefined => "undefined",
                            };
                            print!("COLOR: {:<9?}", color_str);
                            controller.color = color;
                        }
                    }
                };
                if controller.time.elapsed() > std::time::Duration::from_millis(1500) {
                    controller.lift_up_from_color();
                }
            }

            State::LiftUpFromColor if controller.is_arrived(&dobot_pose) => {
                match controller.color {
                    Color::Red => controller.move_to_red(),
                    Color::Green => controller.move_to_green(),
                    Color::Blue => controller.move_to_blue(),
                    Color::Undefined => controller.move_to_mixed(),
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
                        if let Some(Suction::Off) = sample.data {
                            controller.get_ready();
                        }
                    }
                }
            }

            _ => (),
        };

        print!("  STATE: {:<15?}", controller.state);
        print!("  POSE: {:<50}", show_dobot_pose(&dobot_pose));

        if let Some(time_remaining) = LOOP_PERIOD.checked_sub(start.elapsed()) {
            std::thread::sleep(time_remaining);
            print!("  Ts: {:?}", time_remaining)
        } else {
            print!("  Ts: CPU overload")
        }

        print!("\r");
        stdout().flush().unwrap();
    }
}
