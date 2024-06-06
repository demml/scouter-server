use anyhow::{Error, Result};
use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
pub struct Consumer {
    consumer: Consumer,
}

impl ScouterConsumer {
    pub fn new() -> Result<Self, Error> {
        let broker = std::env::var("KAFKA_BROKER").with_context(|| "KAFKA_BROKER is not set")?;

        let consumer = Consumer::from_hosts(vec!["localhost:9092".to_owned()])
            .with_topic("monitoring".to_owned())
            .with_group("scouter".to_owned())
            .with_fallback_offset(FetchOffset::Earliest)
            .with_offset_storage(Some(GroupOffsetStorage::Kafka))
            .create()
            .unwrap();

        Self { consumer }
    }

    pub fn poll(&mut self) -> Vec<String> {
        let mut messages = Vec::new();
        for ms in self.consumer.poll().unwrap().iter() {
            for m in ms.messages() {
                let value = String::from_utf8_lossy(m.value);
                messages.push(value.to_string());
            }
            self.consumer.consume_messageset(ms).unwrap();
        }
        self.consumer.commit_consumed().unwrap();

        messages
    }
}
