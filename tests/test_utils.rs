use anyhow::Error;
use chrono::format::DelayedFormat;
use rdkafka::config::ClientConfig;
use rdkafka::error::KafkaError;
use rdkafka::producer::DefaultProducerContext;
use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use rdkafka::producer::Producer;
use scouter_server::api::setup::create_db_pool;
use scouter_server::kafka::consumer::start_kafka_background_poll;
use scouter_server::kafka::consumer::MessageHandler;
use scouter_server::sql::postgres::PostgresClient;
use scouter_server::sql::schema::DriftRecord;
use sqlx::Pool;
use sqlx::Postgres;
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
            let feature_names = vec!["feature0", "feature1", "feature2"];

            for feature_name in feature_names {
                let record = DriftRecord {
                    created_at: chrono::Utc::now().naive_utc(),
                    name: "test_app".to_string(),
                    repository: "test".to_string(),
                    feature: feature_name.to_string(),
                    value: i as f64,
                    version: "1.0.0".to_string(),
                };

                let record_string = serde_json::to_string(&record).unwrap();

                let delivery_status = producer
                    .send(
                        FutureRecord::to(topic_name)
                            .payload(&record_string)
                            .key("Key"),
                        Duration::from_secs(0),
                    )
                    .await;
            }

            // This will be executed when the result is received.
            info!("Delivery status for message {} received", i);
            i
        })
        .collect::<Vec<_>>();

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

pub async fn setup_pool_and_clean_db() -> Result<Pool<Postgres>, Error> {
    // set the postgres database url
    env::set_var(
        "DATABASE_URL",
        "postgresql://postgres:admin@localhost:5432/monitor?",
    );

    // set the max connections for the postgres pool
    env::set_var("MAX_CONNECTIONS", "10");
    let pool = create_db_pool(None).await.expect("error");

    // run migrations
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    sqlx::raw_sql(
        r#"
            DELETE 
            FROM scouter.drift  
            WHERE name = 'test_app'
            "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    Ok(pool)
}

#[allow(dead_code)]
pub async fn produce_message(message: &str, producer: &FutureProducer) -> Result<(), KafkaError> {
    producer
        .send(
            FutureRecord::to("scouter_monitoring")
                .payload(message)
                .key("Key"),
            Duration::from_secs(0),
        )
        .await
        .unwrap();
    Ok(())
}

#[allow(dead_code)]
pub async fn setup_for_api() -> Result<
    (
        PostgresClient,
        tokio::task::JoinHandle<()>,
        tokio::task::JoinHandle<()>,
    ),
    Error,
> {
    let pool = setup_pool_and_clean_db().await.unwrap();

    let db_client = PostgresClient::new(pool.clone()).unwrap();

    let message_handler = MessageHandler::Postgres(db_client.clone());

    let producer_task = tokio::spawn(async move {
        let producer: &FutureProducer = &ClientConfig::new()
            .set("bootstrap.servers", "localhost:9092")
            .create()
            .expect("Producer creation error");
        let feature_names = vec!["feature0", "feature1", "feature2"];

        for feature_name in feature_names {
            for i in 0..100 {
                let record = DriftRecord {
                    created_at: chrono::Utc::now().naive_utc(),
                    name: "test_app".to_string(),
                    repository: "test".to_string(),
                    feature: feature_name.to_string(),
                    value: i as f64,
                    version: "1.0.0".to_string(),
                };

                let record_string = serde_json::to_string(&record).unwrap();
                produce_message(&record_string, producer).await.unwrap();
            }
        }
    });

    let consumer_task = tokio::spawn(async move {
        start_kafka_background_poll(
            message_handler,
            "scouter".to_string(),
            "localhost:9092".to_string(),
            vec!["scouter_monitoring".to_string()],
            None,
            None,
            None,
            None,
        )
        .await;
    });

    // wait for 5 seconds
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    Ok((db_client, producer_task, consumer_task))
}

#[allow(dead_code)]
pub async fn teardown() -> Result<(), Error> {
    // clear the database

    let pool = setup_pool_and_clean_db().await.unwrap();

    sqlx::raw_sql(
        r#"
            DELETE 
            FROM scouter.drift  
            WHERE name = 'test_app'
            "#,
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    Ok(())
}
