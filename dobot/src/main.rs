mod dobot;

use dobot::{base::{CommandID, Dobot}, message::DobotMessage};
use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listeners::NoOpListener,
        qos::{DataReaderQos, QosKind},
        qos_policy::{ReliabilityQosPolicy, ReliabilityQosPolicyKind},
        status::NO_STATUS,
        time::DurationKind,
    },
    publication::data_writer::DataWriter,
    subscription::data_reader::DataReader,
    topic_definition::type_support::{DdsDeserialize, DdsSerialize},
};
use failure::Fallible;
use std::{
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
use types::{DobotPose, MotorSpeed, Suction};


// ----------------------------------------------------------------------------

const MIN_BELT_SPEED: i32 = 500;
const MAX_BELT_SPEED: i32 = 15000;

const LOOP_PERIOD_MS: u64 = 50;

fn thread_on_update<Foo, Update>(reader: DataReader<Foo>, mut f: Update) -> JoinHandle<()>
where
    Foo: for<'de> DdsDeserialize<'de> + Send + Sync + 'static,
    Update: Send + FnMut(Foo) + 'static,
{
    std::thread::spawn(move || loop {
        if let Ok(sample_data) = reader.take(1, &[], &[], &[]) {
            for sample in sample_data {
                if let Ok(data) = sample.data() {
                    f(data);
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(LOOP_PERIOD_MS));
    })
}

fn thread_publish<Foo, Publish>(writer: DataWriter<Foo>, mut f: Publish) -> JoinHandle<()>
where
    Foo: for<'de> DdsDeserialize<'de> + DdsSerialize + Send + Sync + 'static,
    Publish: Send + FnMut() -> Foo + 'static,
{
    std::thread::spawn(move || loop {
        writer.write(&f(), None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(LOOP_PERIOD_MS));
    })
}

fn set_belt_speed(dobot: &Arc<Mutex<Dobot>>, motor_speed: MotorSpeed) -> Fallible<()> {
    let speed = match motor_speed.speed {
        0 => 0,
        s => s.clamp(MIN_BELT_SPEED, MAX_BELT_SPEED),
    };
    let params = [&[0, 1], &speed.to_le_bytes() as &[u8]].concat();
    let command = DobotMessage::new(CommandID::SetEMotor, false, false, params)?;
    dobot.lock().unwrap().send_command(command)?;

    Ok(())
}

fn move_arm(dobot: &Arc<Mutex<Dobot>>, pose: DobotPose) -> Fallible<()> {
    dobot.lock().unwrap().set_ptp_cmd(
        pose.x,
        pose.y,
        pose.z,
        pose.r,
        dobot::base::Mode::MODE_PTP_MOVJ_XYZ,
    )?;
    Ok(())
}

fn set_suction_cup(dobot: &Arc<Mutex<Dobot>>, suction: Suction) -> Fallible<()> {
    match suction {
        Suction { is_on: true } => dobot.lock().unwrap().set_end_effector_suction_cup(true)?,
        Suction { is_on: false } => dobot.lock().unwrap().set_end_effector_suction_cup(false)?,
    };

    Ok(())
}

fn get_pose(dobot: &Arc<Mutex<Dobot>>) -> Fallible<DobotPose> {
    let pose = dobot.lock().unwrap().get_pose()?;
    Ok(DobotPose {
        x: pose.x,
        y: pose.y,
        z: pose.z,
        r: pose.r,
    })
}

fn main() -> Fallible<()> {
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
                .create_datareader(
                    &topic_conveyor_belt_speed,
                    QosKind::Specific(reliable_reader_qos.clone()),
                    NoOpListener::new(),
                    NO_STATUS,
                )
                .unwrap();
            thread_on_update(belt_speed_reader, move |motor_speed| {
                set_belt_speed(&dobot, motor_speed).unwrap();
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
                .create_datareader(
                    &topic_arm_movement,
                    QosKind::Specific(reliable_reader_qos.clone()),
                    NoOpListener::new(),
                    NO_STATUS,
                )
                .unwrap();
            thread_on_update(arm_movement_reader, move |movement| {
                move_arm(&dobot, movement).unwrap();
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
                .create_datareader(
                    &topic_suction,
                    QosKind::Specific(reliable_reader_qos.clone()),
                    NoOpListener::new(),
                    NO_STATUS,
                )
                .unwrap();
            thread_on_update(suction_reader, move |suction| {
                set_suction_cup(&dobot, suction).unwrap();
                *suction_state.lock().unwrap() = suction;
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
            thread_publish(pose_writer, move || get_pose(&dobot).unwrap())
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
            thread_publish(suction_writer, move || *suction_state.lock().unwrap())
        },
    ];

    dobot.lock().unwrap().set_home().unwrap().wait().unwrap();

    for thread in running_threads {
        thread.join().unwrap();
    }

    Ok(())
}
