use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{qos::QosKind, status::NO_STATUS},
    listener::NO_LISTENER,
};
use rust_gpiozero::InputDevice;
use types::{Color, SensorState};

const COLOR_SENSOR_GPIO1: u8 = 20;
const COLOR_SENSOR_GPIO2: u8 = 21;
const SWITCH_GPIO: u8 = 22;

const LOOP_PERIOD: std::time::Duration = std::time::Duration::from_millis(5);

struct ColorSensor {
    pin1: InputDevice,
    pin2: InputDevice,
}

impl ColorSensor {
    fn new(pin1: InputDevice, pin2: InputDevice) -> Self {
        Self { pin1, pin2 }
    }

    fn value(&self) -> Color {
        match (self.pin1.value(), self.pin2.value()) {
            (false, false) => Color::Undefined,
            (false, true) => Color::Blue,
            (true, false) => Color::Green,
            (true, true) => Color::Red,
        }
    }
}

fn main() {
    let toggle_switch = InputDevice::new(SWITCH_GPIO);

    let color_sensor = ColorSensor::new(
        InputDevice::new_with_pullup(COLOR_SENSOR_GPIO1),
        InputDevice::new_with_pullup(COLOR_SENSOR_GPIO2),
    );

    let domain_id = 0;
    let participant_factory = DomainParticipantFactory::get_instance();
    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic_availability = participant
        .create_topic::<SensorState>(
            "ColorSensorAvailability",
            "SensorState",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let topic_color = participant
        .create_topic::<Color>(
            "ColorSensor",
            "Color",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_availability = publisher
        .create_datawriter(
            &topic_availability,
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let writer_color = publisher
        .create_datawriter(&topic_color, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    loop {
        let start = std::time::Instant::now();

        let is_on = toggle_switch.value();
        let color_sensor_state = SensorState::Available;

        writer_availability.write(color_sensor_state, None).unwrap();

        if is_on {
            let color = color_sensor.value();
            print!("COLOR: {:?}", color);
            writer_color.write(color, None).unwrap();
        }

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
