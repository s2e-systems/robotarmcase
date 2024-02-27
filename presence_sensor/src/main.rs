use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{listeners::NoOpListener, qos::QosKind, status::NO_STATUS},
};
use rust_gpiozero::InputDevice;
use types::{PresenceSensor, SensorState};


const SWITCH_GPIO: u8 = 22;
const SENSOR_GPIO: u8 = 21;

const LOOP_PERIOD: std::time::Duration = std::time::Duration::from_millis(5);

fn main() {
    let domain_id = 0;
    let toggle_switch = InputDevice::new(SWITCH_GPIO);
    let presence_sensor = InputDevice::new_with_pullup(SENSOR_GPIO);

    let participant_factory = DomainParticipantFactory::get_instance();
    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NoOpListener::new(), NO_STATUS)
        .unwrap();

    let topic_availability = participant
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

    let publisher = participant
        .create_publisher(QosKind::Default, NoOpListener::new(), NO_STATUS)
        .unwrap();
    let writer_availability = publisher
        .create_datawriter(
            &topic_availability,
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();
    let writer_presence = publisher
        .create_datawriter(
            &topic_presence,
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();

    loop {
        let start = std::time::Instant::now();

        let availability = SensorState{ is_on: toggle_switch.value() };
        writer_availability.write(&availability, None).unwrap();

        let presence = if availability.is_on {
            let presence = PresenceSensor{ present: presence_sensor.value() };
            writer_presence.write(&presence, None).unwrap();
            Some(presence.present)
        } else {
            None
        };

        print!("AVAILABILITY: {:<6?}  PRESENCE: {:<10?}", availability.is_on, presence);

        if let Some(time_remaining) = LOOP_PERIOD.checked_sub(start.elapsed()) {
            std::thread::sleep(time_remaining);
            print!("  REMAINING TIME: {:?}", time_remaining)
        } else {
            print!("  REMAINING TIME: CPU overload")
        }
        print!("\r");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
    }
}
