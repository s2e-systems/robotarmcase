mod dobot;

use dobot::{
    base::{CommandID, Dobot},
    message::DobotMessage,
};
use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        qos::{DataReaderQos, DataWriterQos, QosKind},
        qos_policy::{ReliabilityQosPolicy, ReliabilityQosPolicyKind},
        sample_info::{ANY_INSTANCE_STATE, ANY_VIEW_STATE, SampleStateKind},
        status::NO_STATUS,
        time::DurationKind,
        type_support::TypeSupport,
    },
    listener::NO_LISTENER,
};
use std::{io::Write, time::Instant};
use types::{ConveyorBeltSpeed, DobotPose, Suction};

const MIN_BELT_SPEED: i32 = 500;
const MAX_BELT_SPEED: i32 = 15000;

const LOOP_PERIOD: std::time::Duration = std::time::Duration::from_millis(20);

fn speed_to_command_bytes(speed: i32) -> Vec<u8> {
    let speed = match speed {
        0 => 0,
        s => s.clamp(MIN_BELT_SPEED, MAX_BELT_SPEED),
    };
    [&[0, 1], speed.to_le_bytes().as_slice()].concat()
}

fn main() -> Result<(), dobot::error::Error> {
    let domain_id = 0;

    let mut dobot = Dobot::open().unwrap();
    let mut suction_state = Suction::Off;

    let reliable_reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Infinite,
        },
        ..Default::default()
    };

    let participant_factory = DomainParticipantFactory::get_instance();
    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
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
    let belt_speed_reader = subscriber
        .create_datareader::<ConveyorBeltSpeed>(
            &topic_conveyor_belt_speed,
            QosKind::Specific(reliable_reader_qos.clone()),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let topic_arm_movement = participant
        .create_topic::<DobotPose>(
            "DobotArmMovement",
            DobotPose::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let arm_movement_reader = subscriber
        .create_datareader::<DobotPose>(
            &topic_arm_movement,
            QosKind::Specific(reliable_reader_qos.clone()),
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
    let suction_reader = subscriber
        .create_datareader::<Suction>(
            &topic_suction,
            QosKind::Specific(reliable_reader_qos.clone()),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let topic_robot_pose = participant
        .create_topic::<DobotPose>(
            "CurrentDobotPose",
            DobotPose::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let pose_writer = publisher
        .create_datawriter(&topic_robot_pose, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic_current_suction = participant
        .create_topic::<Suction>(
            "CurrentSuctionCupState",
            Suction::get_type_name(),
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let suction_writer = publisher
        .create_datawriter(
            &topic_current_suction,
            QosKind::Specific(DataWriterQos {
                reliability: ReliabilityQosPolicy {
                    kind: ReliabilityQosPolicyKind::Reliable,
                    max_blocking_time: DurationKind::Infinite,
                },
                ..Default::default()
            }),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let params = speed_to_command_bytes(0);
    let command = DobotMessage::new(CommandID::SetEMotor, false, false, params).unwrap();
    dobot.send_command(command).unwrap();
    dobot.set_end_effector_suction_cup(Suction::Off).unwrap();
    dobot.set_home().unwrap().wait().unwrap();

    loop {
        let start = Instant::now();

        if let Ok(sample_data) = belt_speed_reader.read(
            1,
            &[SampleStateKind::NotRead],
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        ) {
            for sample in sample_data {
                if let Some(motor_speed) = sample.data {
                    let params = speed_to_command_bytes(motor_speed.speed);
                    let command =
                        DobotMessage::new(CommandID::SetEMotor, false, false, params).unwrap();
                    dobot.send_command(command).unwrap();
                }
            }
        }

        if let Ok(sample_data) = arm_movement_reader.read(
            1,
            &[SampleStateKind::NotRead],
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        ) {
            for sample in sample_data {
                if let Some(pose) = sample.data {
                    dobot
                        .set_ptp_cmd(
                            pose.x,
                            pose.y,
                            pose.z,
                            pose.r,
                            dobot::base::Mode::MODE_PTP_MOVJ_XYZ,
                        )
                        .unwrap();
                }
            }
        }

        if let Ok(sample_data) = suction_reader.read(
            1,
            &[SampleStateKind::NotRead],
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        ) {
            for sample in sample_data {
                if let Some(suction) = sample.data {
                    dobot.set_end_effector_suction_cup(suction).unwrap();
                    suction_state = suction;
                }
            }
        }

        let pose = dobot.get_pose().unwrap();
        let dobot_pose = DobotPose {
            x: pose.x,
            y: pose.y,
            z: pose.z,
            r: pose.r,
        };

        pose_writer.write(dobot_pose, None).unwrap();
        suction_writer.write(suction_state, None).unwrap();

        print!("POSE: {:<50} SUCTION: {:<4}", dobot_pose, suction_state);
        if let Some(time_remaining) = LOOP_PERIOD.checked_sub(start.elapsed()) {
            std::thread::sleep(time_remaining);
            print!("  REMAINING TIME: {:?}", time_remaining)
        } else {
            print!("  REMAINING TIME: CPU overload")
        }
        print!("\r");
        std::io::stdout().flush().unwrap();
    }
}
