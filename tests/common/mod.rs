use anyhow::Error;

use kafka::producer::{Producer, Record, RequiredAcks};
use scouter_server::kafka::consumer::ScouterConsumer;
use scouter_server::sql::postgres::PostgresClient;
use scouter_server::sql::schema::DriftRecord;
use std::env;
use std::time::Duration;

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
            WHERE service_name = 'test_app'
            "#,
    )
    .fetch_all(&client.pool)
    .await
    .unwrap();

    Ok(client)
}

pub async fn setup_kafka() -> Result<(ScouterConsumer, Producer), Error> {
    // set the kafka broker address
    env::set_var("KAFKA_BROKER", "localhost:9092");

    // set the kafka topic
    env::set_var("KAFKA_TOPIC", "scouter_monitoring");

    let consumer = ScouterConsumer::new().unwrap();

    // producer for testing
    let producer = Producer::from_hosts(vec!["localhost:9092".to_owned()])
        .with_ack_timeout(Duration::from_secs(1))
        .with_required_acks(RequiredAcks::One)
        .create()
        .unwrap();

    Ok((consumer, producer))
}

pub async fn setup() -> Result<(PostgresClient, ScouterConsumer, Producer), Error> {
    // setup db and kafka

    let db_client = setup_db().await?;
    let (consumer, producer) = setup_kafka().await?;

    Ok((db_client, consumer, producer))
}

pub async fn setup_for_api() -> Result<PostgresClient, Error> {
    // setup db

    let (db_client, mut scouter_consumer, mut producer) = setup().await.unwrap();

    // feature name vec
    let feature_names = vec!["feature1", "feature2", "feature3"];

    // for each features, generate 1000 records
    feature_names.iter().for_each(|feature_name| {
        for i in 0..1000 {
            let record = DriftRecord {
                created_at: Some(chrono::Utc::now().naive_utc()),
                service_name: "test_app".to_string(),
                feature: feature_name.to_string(),
                value: i as f64,
                version: "1.0.0".to_string(),
            };

            producer
                .send(&Record::from_value(
                    "scouter_monitoring",
                    serde_json::to_string(&record).unwrap().as_bytes(),
                ))
                .unwrap();
        }
    });
    let not_empty = true;

    while not_empty {
        let messages = scouter_consumer.get_messages().await.unwrap();
        if messages.is_empty() {
            break;
        }
        scouter_consumer
            .insert_messages(messages, &db_client)
            .await
            .unwrap();
    }

    let results = db_client
        .raw_query(
            r#"
        SELECT * 
        FROM scouter.drift  
        WHERE service_name = 'test_app'
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
            WHERE service_name = 'test_app'
            "#,
    )
    .fetch_all(&db_client.pool)
    .await
    .unwrap();

    Ok(())
}
