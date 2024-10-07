mod test_utils;

#[cfg(feature = "kafka")]
mod kafka_integration {

    use anyhow::Error;

    use scouter::core::drift::base::{ServerRecord, ServerRecords};
    use scouter::core::drift::spc::types::SpcServerRecord;
    use scouter::core::observe::observer::{LatencyMetrics, ObservabilityMetrics, RouteMetrics};
    use scouter_server::sql::postgres::PostgresClient;

    use std::collections::HashMap;
    use std::env;

    use rdkafka::config::ClientConfig;
    use rdkafka::producer::FutureProducer;
    use rdkafka::producer::FutureRecord;
    use rdkafka::producer::Producer;
    use std::time::Duration;

    use scouter_server::consumer::kafka::startup::kafka_startup::startup_kafka;

    use crate::test_utils;

    pub async fn populate_topic_spc(topic_name: &str) -> Result<(), Error> {
        // Produce some messages

        let kafka_brokers =
            env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_owned());
        let producer: &FutureProducer = &ClientConfig::new()
            .set("bootstrap.servers", &kafka_brokers)
            .set("statistics.interval.ms", "500")
            .set("api.version.request", "true")
            .set("debug", "all")
            .set("message.timeout.ms", "30000")
            .create()
            .expect("Producer creation error");

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

                let server_records = ServerRecords {
                    record_type: scouter::core::drift::base::RecordType::SPC,
                    records: vec![record],
                };

                let record_string = serde_json::to_string(&server_records).unwrap();

                let produce_future = producer.send(
                    FutureRecord::to(topic_name)
                        .payload(&record_string)
                        .key("Key"),
                    Duration::from_secs(1),
                );

                match produce_future.await {
                    Ok(delivery) => println!("Sent: {:?}", delivery),
                    Err((e, _)) => println!("Error: {:?}", e),
                }
            }
        }
        producer.flush(Duration::from_secs(1)).unwrap();
        Ok(())
    }

    pub async fn populate_topic_observability(topic_name: &str) -> Result<(), Error> {
        // Produce some messages

        let kafka_brokers =
            env::var("KAFKA_BROKERS").unwrap_or_else(|_| "localhost:9092".to_owned());
        let producer: &FutureProducer = &ClientConfig::new()
            .set("bootstrap.servers", &kafka_brokers)
            .set("statistics.interval.ms", "500")
            .set("api.version.request", "true")
            .set("debug", "all")
            .set("message.timeout.ms", "30000")
            .create()
            .expect("Producer creation error");

        for i in 0..15 {
            // The send operation on the topic returns a future, which will be
            // completed once the result or failure from Kafka is received.
            let mut status_codes = HashMap::new();
            status_codes.insert(200 as usize, 10 as i64);

            let latency = LatencyMetrics {
                p5: 0 as f64,
                p25: 0 as f64,
                p50: 0.25 as f64,
                p95: 0.25 as f64,
                p99: 0.25 as f64,
            };

            let route_metrics = RouteMetrics {
                route_name: "test_route".to_string(),
                metrics: latency,
                request_count: 10,
                error_count: 0,
                error_latency: 0 as f64,
                status_codes: status_codes,
            };
            let record = ObservabilityMetrics {
                name: "test_app".to_string(),
                repository: "test_repo".to_string(),
                version: "1.0.0".to_string(),
                request_count: i,
                error_count: i,
                route_metrics: vec![route_metrics],
            };

            let server_record = ServerRecord::OBSERVABILITY { record: record };

            let server_records = ServerRecords {
                record_type: scouter::core::drift::base::RecordType::OBSERVABILITY,
                records: vec![server_record],
            };

            let record_string = serde_json::to_string(&server_records).unwrap();

            let produce_future = producer.send(
                FutureRecord::to(topic_name)
                    .payload(&record_string)
                    .key("Key"),
                Duration::from_secs(1),
            );
            let record_string = serde_json::to_string(&server_records).unwrap();

            match produce_future.await {
                Ok(delivery) => println!("Sent: {:?}", delivery),
                Err((e, _)) => println!("Error: {:?}", e),
            }
        }

        producer.flush(Duration::from_secs(1)).unwrap();
        Ok(())
    }

    #[tokio::test]
    #[ignore]
    async fn test_api_with_kafka() {
        // setup resources
        let topic_name = "scouter_monitoring";
        let pool = test_utils::setup_db(true).await.unwrap();
        let db_client = PostgresClient::new(pool.clone()).unwrap();

        let startup = startup_kafka(pool.clone());

        match startup.await {
            Ok(_) => println!("Successfully started kafka"),
            Err(e) => println!("Error starting kafka: {:?}", e),
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(7)).await;

        // populate kafka topic (15 messages)
        let test = populate_topic_spc(topic_name);
        match test.await {
            Ok(_) => println!("Successfully populated kafka topic"),
            Err(e) => println!("Error populating kafka topic: {:?}", e),
        }

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
}
