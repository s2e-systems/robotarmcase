use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{listeners::NoOpListener, qos::QosKind, status::NO_STATUS},
};
use rust_gpiozero::InputDevice;

include!("../../target/idl/types.rs");

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
        .create_topic::<robot_arm_case::Availability>(
            "PresenceSensorAvailability",
            "Availability",
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();
    let topic_presence = participant
        .create_topic::<robot_arm_case::Presence>(
            "PresenceSensor",
            "Presence",
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
            robot_arm_case::Availability::Available
        } else {
            robot_arm_case::Availability::NotAvailable
        };

        writer_availability.write(&availability, None).unwrap();

        if availability == robot_arm_caseAvailability::Available {
            let presence = if presence_sensor.value() {
                robot_arm_case::Presence::Present
            } else {
                robot_arm_case::Presence::NotPresent
            };

            writer_presence.write(&presence, None).unwrap();
        }

        std::thread::sleep(std::time::Duration::from_millis(WRITING_PERIOD_MS));
    }
}
