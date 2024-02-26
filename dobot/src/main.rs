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
};
use std::sync::{Arc, Mutex};
use types::{DobotPose, MotorSpeed, Suction};


const MIN_BELT_SPEED: i32 = 500;
const MAX_BELT_SPEED: i32 = 15000;

const LOOP_PERIOD_MS: u64 = 50;

fn main() -> Result<(), dobot::error::Error> {
    let domain_id = 0;

    let dobot = Arc::new(Mutex::new(Dobot::open().unwrap()));
    let suction_state = Arc::new(Mutex::new(Suction { is_on: false }));

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

    let running_threads = vec![
        {
            let dobot = dobot.clone();
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
            std::thread::spawn(move || loop {
                if let Ok(sample_data) = belt_speed_reader.take(1, &[], &[], &[]) {
                    for sample in sample_data {
                        if let Ok(motor_speed) = sample.data() {
                            let speed = match motor_speed.speed {
                                0 => 0,
                                s => s.clamp(MIN_BELT_SPEED, MAX_BELT_SPEED),
                            };
                            let params = [&[0, 1], &speed.to_le_bytes() as &[u8]].concat();
                            let command = DobotMessage::new(CommandID::SetEMotor, false, false, params).unwrap();
                            dobot.lock().unwrap().send_command(command).unwrap();
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(LOOP_PERIOD_MS));
            })
        },
        {
            let dobot = dobot.clone();
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
            std::thread::spawn(move || loop {
                if let Ok(sample_data) = arm_movement_reader.take(1, &[], &[], &[]) {
                    for sample in sample_data {
                        if let Ok(pose) = sample.data() {
                            dobot.lock().unwrap().set_ptp_cmd(
                                pose.x,
                                pose.y,
                                pose.z,
                                pose.r,
                                dobot::base::Mode::MODE_PTP_MOVJ_XYZ,
                            ).unwrap();
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(LOOP_PERIOD_MS));
            })
        },
        {
            let dobot = dobot.clone();
            let suction_state = suction_state.clone();
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
            std::thread::spawn(move || loop {
                if let Ok(sample_data) = suction_reader.take(1, &[], &[], &[]) {
                    for sample in sample_data {
                        if let Ok(suction) = sample.data() {
                            dobot.lock().unwrap().set_end_effector_suction_cup(suction.is_on).unwrap();
                            *suction_state.lock().unwrap() = suction;
                        }
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(LOOP_PERIOD_MS));
            })
        },
        {
            let dobot = dobot.clone();
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
            std::thread::spawn(move || loop {
                let pose = dobot.lock().unwrap().get_pose().unwrap();
                let dobot_pose = DobotPose {
                    x: pose.x,
                    y: pose.y,
                    z: pose.z,
                    r: pose.r,
                };
                println!("pose {:?}", dobot_pose);
                pose_writer.write(&dobot_pose, None).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(LOOP_PERIOD_MS));
            })
        },
        {
            let suction_state = suction_state.clone();
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
            std::thread::spawn(move || loop {
                let suction = *suction_state.lock().unwrap();
                suction_writer.write(&suction, None).unwrap();
                std::thread::sleep(std::time::Duration::from_millis(LOOP_PERIOD_MS));
            })
        },
    ];

    dobot.lock().unwrap().set_home().unwrap().wait().unwrap();

    for thread in running_threads {
        thread.join().unwrap();
    }

    Ok(())
}
