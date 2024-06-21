use anyhow::Error;

use rdkafka::config::ClientConfig;
use rdkafka::producer::FutureProducer;
use rdkafka::producer::FutureRecord;
use scouter_server::kafka::consumer::{MessageHandler, MockInserter, ScouterConsumer};
use scouter_server::sql::postgres::PostgresClient;
use scouter_server::sql::schema::DriftRecord;
use std::env;
use std::time::{Duration, Instant};

pub async fn setup_db() -> Result<PostgresClient, Error> {
    // set the postgres database url
    env::set_var(
        "DATABASE_URL",
        "postgresql://postgres:admin@localhost:5432/monitor?",
    );

    // set the max connections for the postgres pool
    env::set_var("MAX_CONNECTIONS", "10");

    let client = PostgresClient::new(None).await.expect("error");

    sqlx::raw_sql(
        r#"
            DELETE 
            FROM scouter.drift  
            WHERE name = 'test_app'
            "#,
    )
    .fetch_all(&client.pool)
    .await
    .unwrap();

    Ok(client)
}

pub async fn setup_kafka_producer() -> Result<(FutureProducer), Error> {
    // set the kafka broker address
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .create()
        .expect("Producer creation error");

    Ok(producer)
}

pub async fn setup_kafka_consumer() -> Result<ScouterConsumer, Error> {
    env::set_var(
        "DATABASE_URL",
        "postgresql://postgres:admin@localhost:5432/monitor?",
    );

    // set the max connections for the postgres pool
    env::set_var("MAX_CONNECTIONS", "10");

    let client = PostgresClient::new(None).await.expect("error");

    // set the kafka broker address
    let consumer = ScouterConsumer::new(
        MessageHandler::Postgres(client),
        "localhost:9092".to_string(),
        ["scouter_monitoring".to_string()].to_vec(),
        "scouter".to_string(),
        None,
        None,
        None,
        None,
    )
    .await
    .unwrap();

    Ok(consumer)
}

pub async fn setup() -> Result<(PostgresClient, ScouterConsumer, FutureProducer), Error> {
    // setup db and kafka

    let db_client = setup_db().await?;
    let producer = setup_kafka_producer().await?;
    let consumer = setup_kafka_consumer().await?;

    Ok((db_client, consumer, producer))
}

pub async fn setup_for_api() -> Result<PostgresClient, Error> {
    // setup db

    let (db_client, mut scouter_consumer, mut producer) = setup().await.unwrap();

    // feature name vec
    let feature_names = vec!["feature1", "feature2", "feature3"];
    let topic = "scouter_monitoring";

    for feature in feature_names {
        tokio::spawn(async move {
            let producer = setup_kafka_producer().await.unwrap();
            let mut i: usize = 0;

            loop {
                let record = DriftRecord {
                    created_at: chrono::Utc::now().naive_utc(),
                    name: "test_app".to_string(),
                    repository: "test".to_string(),
                    feature: feature.to_string(),
                    value: i as f64,
                    version: "1.0.0".to_string(),
                };

                let record_string = serde_json::to_string(&record).unwrap();
                producer
                    .send_result(
                        FutureRecord::to(topic)
                            .key("1")
                            .payload(record_string.as_str()),
                    )
                    .unwrap()
                    .await
                    .unwrap()
                    .unwrap();

                i += 1;

                if i == 1000 {
                    break;
                }
            }
        });
    }

    let start = Instant::now();
    // warm up for 10s
    while start.elapsed() < Duration::from_secs(5) {
        continue;
    }

    let not_empty = true;

    let mut count = 0;

    while not_empty {
        let messages = scouter_consumer.poll_message().await.unwrap();

        if messages.is_none() {
            break;
        } else {
            count += 1;
        }
    }

    println!("count: {}", count);

    let results = db_client
        .raw_query(
            r#"
        SELECT * 
        FROM scouter.drift  
        WHERE name = 'test_app'
        "#,
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 3000);

    Ok(db_client)
}

#[allow(dead_code)]
pub async fn teardown(db_client: &PostgresClient) -> Result<(), Error> {
    // clear the database

    sqlx::raw_sql(
        r#"
            DELETE 
            FROM scouter.drift  
            WHERE name = 'test_app'
            "#,
    )
    .fetch_all(&db_client.pool)
    .await
    .unwrap();

    Ok(())
}
