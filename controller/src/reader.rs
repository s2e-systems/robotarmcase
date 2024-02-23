use dust_dds::{
    infrastructure::{error::DdsResult, qos::QosKind, status::NO_STATUS},
    subscription::{data_reader::DataReader, subscriber::Subscriber},
    topic_definition::{
        topic::Topic,
        type_support::{DdsDeserialize, DdsType},
    },
};
use std::fmt::Display;
use std::time::{Duration, SystemTime};
use types::{SensorState, Color, PresenceSensor};

// ----------------------------------------------------------------------------

pub struct Reader<T, const READING_EXPIRE_MS: u64 = 100>
where
    T: for<'de> DdsDeserialize<'de> + 'static,
{
    reader: DataReader<T>,
    value: Option<T>,
    last_update: SystemTime,
}

impl<T, const READING_EXPIRE_MS: u64> Reader<T, READING_EXPIRE_MS>
where
    T: for<'de> DdsDeserialize<'de> + 'static,
{
    pub fn new(topic: &Topic, subscriber: &Subscriber) -> DdsResult<Self> {
        Ok(Self {
            reader: subscriber.create_datareader(topic, QosKind::Default, NoOpListener::new(), NO_STATUS)?,
            value: None,
            last_update: SystemTime::now(),
        })
    }

    pub fn value(&self) -> &Option<T> {
        &self.value
    }

    pub fn update(&mut self) {
        let new_value = self
            .reader
            .take(1, &[], &[], &[])
            .ok()
            .and_then(|v| v.into_iter().last())
            .filter(|sample| sample.sample_info.valid_data)
            .and_then(|sample| sample.data);

        if new_value.is_some() {
            self.last_update = std::time::SystemTime::now();
            self.value = new_value;
        } else if SystemTime::now().duration_since(self.last_update).unwrap()
            > Duration::from_millis(READING_EXPIRE_MS)
        {
            self.value = None;
        }
    }
}

pub struct Sensor<T, const READING_EXPIRE_MS: u64 = 100>
where
    T: for<'de> DdsDeserialize<'de> + 'static,
{
    availability: Reader<SensorState, READING_EXPIRE_MS>,
    value: Reader<T, READING_EXPIRE_MS>,
}

impl<T, const R: u64> Sensor<T, R>
where
    T: for<'de> DdsDeserialize<'de> + 'static,
{
    pub fn new(
        topic_availability: &Topic,
        topic_value: &Topic,
        subscriber: &Subscriber,
    ) -> DdsResult<Self> {
        Ok(Self {
            availability: Reader::new(topic_availability, subscriber)?,
            value: Reader::new(topic_value, subscriber)?,
        })
    }

    pub fn value(&self) -> &Option<T> {
        match self.availability.value() {
            Some(SensorState) => self.value.value(),
            _ => &None,
        }
    }

    pub fn update(&mut self) {
        self.availability.update();
        self.value.update();
    }

    pub fn is_offline(&self) -> bool {
        self.availability.value().is_none()
    }
}

impl<const R: u64> Display for Sensor<PresenceSensor, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_offline() {
            write!(f, "offline")?;
        } else {
            match self.value() {
                None => write!(f, "unavailable")?,
                Some(Presence::NotPresent) => write!(f, "nothing")?,
                Some(Presence::Present) => write!(f, "something")?,
            };
        }
        Ok(())
    }
}

impl<const R: u64> Display for Sensor<Color, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_offline() {
            write!(f, "offline")?;
        } else {
            match self.value() {
                None => write!(f, "unavailable")?,
                Some(Color::Blue) => write!(f, "blue")?,
                Some(Color::Green) => write!(f, "green")?,
                Some(Color::Red) => write!(f, "red")?,
                Some(Color::Other) => write!(f, "other color")?,
            };
        }
        Ok(())
    }
}
