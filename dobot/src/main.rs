mod dobot;

use dobot::{
    base::{CommandID, Dobot},
    message::DobotMessage,
};
use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listeners::NoOpListener,
        qos::{DataReaderQos, QosKind},
        qos_policy::{ReliabilityQosPolicy, ReliabilityQosPolicyKind},
        status::NO_STATUS,
        time::DurationKind,
    },
    subscription::sample_info::{SampleStateKind, ANY_INSTANCE_STATE, ANY_VIEW_STATE},
};
use std::{
    io::Write,
    time::Instant,
};
use types::{DobotPose, MotorSpeed, Suction};

const MIN_BELT_SPEED: i32 = 500;
const MAX_BELT_SPEED: i32 = 15000;

const LOOP_PERIOD: std::time::Duration = std::time::Duration::from_millis(20);

fn show_dobot_pose(pose: &DobotPose) -> String {
    format!(
        "{{x: {:.2}, y: {:.2}, z: {:.2}, r: {:}}}",
        pose.x, pose.y, pose.z, pose.r
    )
}

fn speed_to_command_bytes(speed: i32) -> Vec<u8> {
    let speed = match speed {
        0 => 0,
        s => s.clamp(MIN_BELT_SPEED, MAX_BELT_SPEED),
    };
    [&[0, 1], &speed.to_le_bytes() as &[u8]].concat()
}

fn main() -> Result<(), dobot::error::Error> {
    let domain_id = 0;

    let mut dobot = Dobot::open().unwrap();
    let mut suction_state = Suction { is_on: false };

    let reliable_reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Infinite,
        },
        ..Default::default()
    };

    let participant_factory = DomainParticipantFactory::get_instance();
    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NoOpListener::new(), NO_STATUS)
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NoOpListener::new(), NO_STATUS)
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
    let belt_speed_reader = subscriber
        .create_datareader::<MotorSpeed>(
            &topic_conveyor_belt_speed,
            QosKind::Specific(reliable_reader_qos.clone()),
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();

    let topic_arm_movement = participant
        .create_topic::<DobotPose>(
            "DobotArmMovement",
            "DobotPose",
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();
    let arm_movement_reader = subscriber
        .create_datareader::<DobotPose>(
            &topic_arm_movement,
            QosKind::Specific(reliable_reader_qos.clone()),
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
    let suction_reader = subscriber
        .create_datareader::<Suction>(
            &topic_suction,
            QosKind::Specific(reliable_reader_qos.clone()),
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();

    let topic_robot_pose = participant
        .create_topic::<DobotPose>(
            "CurrentDobotPose",
            "DobotPose",
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();
    let pose_writer = publisher
        .create_datawriter(
            &topic_robot_pose,
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();

    let topic_current_suction = participant
        .create_topic::<Suction>(
            "CurrentSuctionCupState",
            "Suction",
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();
    let suction_writer = publisher
        .create_datawriter(
            &topic_current_suction,
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();

    //dobot.set_home().unwrap().wait().unwrap();
    let params = speed_to_command_bytes(0);
    let command = DobotMessage::new(CommandID::SetEMotor, false, false, params).unwrap();
    dobot.send_command(command).unwrap();
    dobot.set_end_effector_suction_cup(false).unwrap();

    loop {
        let start = Instant::now();

        if let Ok(sample_data) = belt_speed_reader.read(
            1,
            &[SampleStateKind::NotRead],
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        ) {
            for sample in sample_data {
                if let Ok(motor_speed) = sample.data() {
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
                if let Ok(pose) = sample.data() {
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
                if let Ok(suction) = sample.data() {
                    dobot.set_end_effector_suction_cup(suction.is_on).unwrap();
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

        pose_writer.write(&dobot_pose, None).unwrap();
        suction_writer.write(&suction_state, None).unwrap();

        print!("POSE: {:<50}", show_dobot_pose(&dobot_pose));
        if let Some(time_remaining) = LOOP_PERIOD.checked_sub(start.elapsed().into()) {
            std::thread::sleep(time_remaining);
            print!("  REMAINING TIME: {:?}", time_remaining)
        } else {
            print!("  REMAINING TIME: CPU overload")
        }
        print!("\r");
        std::io::stdout().flush().unwrap();
    }
    Ok(())
}
