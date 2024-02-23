use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{listeners::NoOpListener, qos::QosKind, status::NO_STATUS},
};
use rust_gpiozero::InputDevice;
use types::{PresenceSensor, SensorState};


// ----------------------------------------------------------------------------

const SWITCH_GPIO: u8 = 22;
const SENSOR_GPIO: u8 = 21;

const WRITING_PERIOD_MS: u64 = 50;

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
            "SensorStates",
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();
    let topic_presence = participant
        .create_topic::<PresenceSensor>(
            "PresenceSensor",
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
        let availability = if toggle_switch.value() {
            SensorState{ is_on: true }
        } else {
            SensorState{ is_on: false }
        };

        writer_availability.write(&availability, None).unwrap();

        let available = SensorState{ is_on: true };
        if availability == available {
            let presence = if presence_sensor.value() {
                SensorState{ is_on: true }
            } else {
                SensorState{ is_on: false }
            };

            writer_presence.write(&presence, None).unwrap();
        }

        std::thread::sleep(std::time::Duration::from_millis(WRITING_PERIOD_MS));
    }
}
