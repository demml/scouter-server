use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use kafka::producer::{Producer, Record, RequiredAcks};
use std::fmt::Write;
use std::time::Duration;

fn main() {
    //et mut producer = Producer::from_hosts(vec!["localhost:9092".to_owned()])
    //   .with_ack_timeout(Duration::from_secs(1))
    //   .with_required_acks(RequiredAcks::One)
    //   .create()
    //   .unwrap();

    //et mut buf = String::with_capacity(10);
    //or i in 0..10 {
    //   write!(&mut buf, "{} {}", "test", i).unwrap(); // some computation of the message data to be sent
    //   producer
    //       .send(&Record::from_value("monitoring", buf.as_bytes()))
    //       .unwrap();
    //   buf.clear();
    //

    let mut consumer = Consumer::from_hosts(vec!["localhost:9092".to_owned()])
        .with_topic("monitoring".to_owned())
        .with_group("scouter".to_owned())
        .with_fallback_offset(FetchOffset::Earliest)
        .with_offset_storage(Some(GroupOffsetStorage::Kafka))
        .create()
        .unwrap();

    loop {
        for ms in consumer.poll().unwrap().iter() {
            for m in ms.messages() {
                // deserialize the message data
                let value = String::from_utf8_lossy(m.value);
                println!("{:?}", value);
            }
            consumer.consume_messageset(ms).unwrap();
        }
        consumer.commit_consumed().unwrap();
    }
}
