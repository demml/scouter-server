mod test_utils;
#[cfg(feature = "rabbitmq")]
mod rabbit_integration {

    use anyhow::Error;
    use lapin::BasicProperties;
    use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties};
    use scouter::core::drift::base::{ServerRecord, ServerRecords};
    use scouter::core::drift::spc::types::SpcServerRecord;
    use scouter::core::observe::observer::{LatencyMetrics, ObservabilityMetrics, RouteMetrics};
    use scouter_server::consumer::rabbitmq::startup::rabbitmq_startup::startup_rabbitmq;
    use scouter_server::sql::postgres::PostgresClient;
    use std::collections::HashMap;

    use crate::test_utils;

    pub async fn populate_rabbitmq_observability() -> Result<(), Error> {
        // Produce some messages

        let rabbit_addr = std::env::var("RABBITMQ_ADDR")
            .unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672/%2f".into());

        let conn = Connection::connect(&rabbit_addr, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await.unwrap();
        channel
            .queue_declare(
                "scouter_monitoring",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        for i in 0..15 {
            // The send operation on the topic returns a future, which will be
            // completed once the result or failure from Kafka is received.
            let mut status_codes = HashMap::new();
            status_codes.insert(200_usize, 10_i64);

            let latency = LatencyMetrics {
                p5: 0_f64,
                p25: 0_f64,
                p50: 0.25_f64,
                p95: 0.25_f64,
                p99: 0.25_f64,
            };

            let route_metrics = RouteMetrics {
                route_name: "test_route".to_string(),
                metrics: latency,
                request_count: 10,
                error_count: 0,
                error_latency: 0 as f64,
                status_codes,
            };
            let record = ObservabilityMetrics {
                name: "test_app".to_string(),
                repository: "test_repo".to_string(),
                version: "1.0.0".to_string(),
                request_count: i,
                error_count: i,
                route_metrics: vec![route_metrics],
            };

            let server_record = ServerRecord::OBSERVABILITY { record };

            let server_records = ServerRecords {
                record_type: scouter::core::drift::base::RecordType::OBSERVABILITY,
                records: vec![server_record],
            };

            let _confirm = channel
                .basic_publish(
                    "",
                    "scouter_monitoring",
                    BasicPublishOptions::default(),
                    &serde_json::to_string(&server_records).unwrap().into_bytes(),
                    BasicProperties::default(),
                )
                .await?;
        }

        Ok(())
    }

    pub async fn populate_rabbit_spc_queue() -> Result<(), Error> {
        // Produce some messages

        let rabbit_addr = std::env::var("RABBITMQ_ADDR")
            .unwrap_or_else(|_| "amqp://guest:guest@127.0.0.1:5672/%2f".into());

        let conn = Connection::connect(&rabbit_addr, ConnectionProperties::default()).await?;
        let channel = conn.create_channel().await.unwrap();
        channel
            .queue_declare(
                "scouter_monitoring",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        for i in 0..15 {
            // The send operation on the topic returns a future, which will be
            // completed once the result or failure from Kafka is received.
            let feature_names = vec!["feature0", "feature1", "feature2"];

            for feature_name in feature_names {
                let record = ServerRecord::SPC {
                    record: SpcServerRecord {
                        created_at: chrono::Utc::now().naive_utc(),
                        name: "test_app".to_string(),
                        repository: "test".to_string(),
                        feature: feature_name.to_string(),
                        value: i as f64,
                        version: "1.0.0".to_string(),
                    },
                };

                // treat each record as a separate message
                let server_records = ServerRecords {
                    record_type: scouter::core::drift::base::RecordType::SPC,
                    records: vec![record],
                };

                let record_string = serde_json::to_string(&server_records).unwrap().into_bytes();

                let _confirm = channel
                    .basic_publish(
                        "",
                        "scouter_monitoring",
                        BasicPublishOptions::default(),
                        &record_string,
                        BasicProperties::default(),
                    )
                    .await?;
            }
        }

        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_api_with_rabbitmq() {
        // setup resources
        let pool = test_utils::setup_db(true).await.unwrap();
        let db_client = PostgresClient::new(pool.clone()).unwrap();

        let startup = startup_rabbitmq(pool.clone());

        match startup.await {
            Ok(_) => println!("Successfully started rabbitmq consumer"),
            Err(e) => println!("Error starting rabbitmq consumer: {:?}", e),
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // populate rabbit queue (15 messages)
        populate_rabbit_spc_queue().await.unwrap();

        // sleep for 5 seconds to allow kafka to process messages
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let result = db_client
            .raw_query(
                r#"
                    SELECT *
                    FROM scouter.drift
                    WHERE name = 'test_app'
                    LIMIT 10
                    "#,
            )
            .await
            .unwrap();

        let count = result.len();

        assert_eq!(count, 10);
    }

    #[tokio::test]
    #[ignore]
    async fn test_rabbitmq_observability() {
        // setup resources
        let pool = test_utils::setup_db(true).await.unwrap();
        let db_client = PostgresClient::new(pool.clone()).unwrap();

        let startup = startup_rabbitmq(pool.clone());

        match startup.await {
            Ok(_) => println!("Successfully started rabbitmq consumer"),
            Err(e) => println!("Error starting rabbitmq consumer: {:?}", e),
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // populate rabbit queue (15 messages)
        populate_rabbitmq_observability().await.unwrap();

        // sleep for 5 seconds to allow kafka to process messages
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        let result = db_client
            .raw_query(
                r#"
                    SELECT *
                    FROM scouter.observability_metrics
                    WHERE name = 'test_app'
                    LIMIT 10
                    "#,
            )
            .await
            .unwrap();

        let count = result.len();

        assert_eq!(count, 10);
    }
}
