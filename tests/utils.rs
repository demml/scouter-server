use rdkafka::config::ClientConfig;
use rdkafka::producer::DefaultProducerContext;
use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use rdkafka::producer::Producer;
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use tracing::info;

pub async fn populate_topic(topic_name: &str) {
    // Produce some messages

    let kafka_broker = env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:9092".to_owned());
    let producer: &FutureProducer = &ClientConfig::new()
        .set("bootstrap.servers", &kafka_broker)
        .set("statistics.interval.ms", "500")
        .set("api.version.request", "true")
        .set("debug", "all")
        .set("message.timeout.ms", "30000")
        .create()
        .expect("Producer creation error");

    let futures = (0..5)
        .map(|i| async move {
            // The send operation on the topic returns a future, which will be
            // completed once the result or failure from Kafka is received.
            let delivery_status = producer
                .send(
                    FutureRecord::to(topic_name)
                        .payload(&format!("Message {}", i))
                        .key(&format!("Key {}", i)),
                    Duration::from_secs(0),
                )
                .await;

            // This will be executed when the result is received.
            info!("Delivery status for message {} received", i);
            delivery_status
        })
        .collect::<Vec<_>>();

    for future in futures {
        println!("Future completed. Result: {:?}", future.await);
    }

    producer.flush(Duration::from_secs(1)).unwrap()
}

pub fn consumer_config(
    group_id: &str,
    config_overrides: Option<HashMap<&str, &str>>,
) -> ClientConfig {
    let kafka_broker = env::var("KAFKA_BROKER").unwrap_or_else(|_| "localhost:9092".to_owned());

    let mut config = ClientConfig::new();

    config.set("group.id", group_id);
    config.set("client.id", "rdkafka_integration_test_client");
    config.set("bootstrap.servers", &kafka_broker);
    config.set("enable.partition.eof", "false");
    config.set("session.timeout.ms", "6000");
    config.set("enable.auto.commit", "false");
    config.set("statistics.interval.ms", "500");
    config.set("api.version.request", "true");
    config.set("debug", "all");
    config.set("auto.offset.reset", "earliest");

    if let Some(overrides) = config_overrides {
        for (key, value) in overrides {
            config.set(key, value);
        }
    }

    config
}
