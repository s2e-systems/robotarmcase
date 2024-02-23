use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{listeners::NoOpListener, qos::QosKind, status::NO_STATUS},
};
use rust_gpiozero::InputDevice;
use types::{Color, SensorState};

// ----------------------------------------------------------------------------

const COLOUR_SENSOR_GPIO1: u8 = 20;
const COLOUR_SENSOR_GPIO2: u8 = 21;
const SWITCH_GPIO: u8 = 22;

const WRITING_PERIOD_MS: u64 = 50;

struct ColourSensor {
    pin1: InputDevice,
    pin2: InputDevice,
}

impl ColourSensor {
    fn new(pin1: InputDevice, pin2: InputDevice) -> Self {
        Self { pin1, pin2 }
    }

    fn value(&self) -> Color {
        match (self.pin1.value(), self.pin2.value()) {
            (false, false) => Color {
                red: 0,
                green: 0,
                blue: 0,
            },
            (false, true) => Color {
                red: 0,
                green: 0,
                blue: 255,
            },
            (true, false) => Color {
                red: 0,
                green: 255,
                blue: 0,
            },
            (true, true) => Color {
                red: 255,
                green: 0,
                blue: 0,
            },
        }
    }
}

fn main() {
    let toggle_switch = InputDevice::new(SWITCH_GPIO);

    let colour_sensor = ColourSensor::new(
        InputDevice::new_with_pullup(COLOUR_SENSOR_GPIO1),
        InputDevice::new_with_pullup(COLOUR_SENSOR_GPIO2),
    );

    let domain_id = 0;
    let participant_factory = DomainParticipantFactory::get_instance();
    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NoOpListener::new(), NO_STATUS)
        .unwrap();

    let topic_availability = participant
        .create_topic::<SensorState>(
            "ColorSensorState",
            "SensorState",
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();
    let topic_colour = participant
        .create_topic::<Color>(
            "ColorSensor",
            "Color",
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
    let writer_colour = publisher
        .create_datawriter(
            &topic_colour,
            QosKind::Default,
            NoOpListener::new(),
            NO_STATUS,
        )
        .unwrap();

    loop {
        let is_on = toggle_switch.value();
        let colour_sensor_state = SensorState { is_on };

        writer_availability
            .write(&colour_sensor_state, None)
            .unwrap();

        if is_on {
            writer_colour.write(&colour_sensor.value(), None).unwrap();
        }

        std::thread::sleep(std::time::Duration::from_millis(WRITING_PERIOD_MS));
    }
}
