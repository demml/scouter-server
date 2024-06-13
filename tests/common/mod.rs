use anyhow::Error;
use kafka::producer::{Producer, Record, RequiredAcks};
use scouter_server::kafka::consumer::ScouterConsumer;
use scouter_server::sql::postgres::PostgresClient;
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

    let client = PostgresClient::new().await.expect("error");

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
