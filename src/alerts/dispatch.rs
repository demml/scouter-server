use anyhow::{Context, Result};
use async_trait::async_trait;
use scouter::utils::types::FeatureAlerts;
use serde_json::{json, Value};
use std::env;

trait DispatchHelpers {
    fn construct_alert_description(feature_alerts: &FeatureAlerts) -> String {
        let mut alert_description = String::new();
        for (i, (_, feature_alert)) in feature_alerts.features.iter().enumerate() {
            if feature_alert.alerts.is_empty() {
                continue;
            }
            if i == 0 {
                alert_description.push_str("Features that have drifted \n");
            }
            alert_description.push_str(&format!("{} alerts: \n", &feature_alert.feature));
            feature_alert.alerts.iter().for_each(|alert| {
                alert_description.push_str(&format!(
                    "alert kind {} -- alert zone: {} \n",
                    &alert.kind, &alert.zone
                ))
            });
        }
        alert_description
    }
}

#[async_trait]
pub trait DispatchAsync {
    async fn process_alerts(&self, feature_alerts: &FeatureAlerts, model_name: &str) -> Result<()>;
}

#[derive(Debug)]
pub struct OpsGenieAlertDispatcher {
    ops_genie_api_url: String,
    ops_genie_api_key: String,
    http_client: reqwest::Client,
}

impl Default for OpsGenieAlertDispatcher {
    fn default() -> Self {
        Self {
            ops_genie_api_url: env::var("OPSGENIE_API_URL").unwrap_or("api_url".to_string()),
            ops_genie_api_key: env::var("OPSGENIE_API_KEY").unwrap_or("api_key".to_string()),
            http_client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl DispatchAsync for OpsGenieAlertDispatcher {
    async fn process_alerts(&self, feature_alerts: &FeatureAlerts, model_name: &str) -> Result<()> {
        let alert_description = Self::construct_alert_description(feature_alerts);

        if !alert_description.is_empty() {
            let alert_body = Self::construct_alert_body(&alert_description, model_name);
            self.send_alerts(alert_body).await?;
        }
        Ok(())
    }
}

impl DispatchHelpers for OpsGenieAlertDispatcher {}

impl OpsGenieAlertDispatcher {
    fn construct_alert_body(alert_description: &str, model_name: &str) -> Value {
        json!(
                {
                    "message": format!("Model drift detected for {}", model_name),
                    "description": alert_description,
                    "responders":[
                        {"name":"ds-team", "type":"team"}
                    ],
                    "visibleTo":[
                        {"name":"ds-team", "type":"team"}
                    ],
                    "tags": ["Model Drift"],
                    "priority": "P1"
                }
        )
    }

    async fn send_alerts(&self, body: Value) -> Result<()> {
        self.http_client
            .post(format!("{}/alerts", &self.ops_genie_api_url))
            .header(
                "Authorization",
                format!("GenieKey {}", &self.ops_genie_api_key),
            )
            .json(&body)
            .send()
            .await
            .with_context(|| "Error posting alert to opsgenie")?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ConsoleAlertDispatcher;

#[async_trait]
impl DispatchAsync for ConsoleAlertDispatcher {
    async fn process_alerts(&self, feature_alerts: &FeatureAlerts, model_name: &str) -> Result<()> {
        let alert_description = Self::construct_alert_description(feature_alerts);

        if !alert_description.is_empty() {
            Self::send_alerts(&alert_description, model_name);
        }
        Ok(())
    }
}

impl DispatchHelpers for ConsoleAlertDispatcher {}

impl ConsoleAlertDispatcher {
    fn send_alerts(alert_description: &str, model_name: &str) {
        println!(
            "{} is experiencing drift. \n{}",
            model_name, alert_description
        );
    }
}

#[derive(Debug)]
pub enum AlertDispatcher {
    Console(ConsoleAlertDispatcher),
    OpsGenie(OpsGenieAlertDispatcher),
}

impl AlertDispatcher {
    pub async fn process_alerts(
        &self,
        feature_alerts: &FeatureAlerts,
        model_name: &str,
    ) -> Result<()> {
        match self {
            AlertDispatcher::Console(dispatcher) => {
                dispatcher.process_alerts(feature_alerts, model_name).await
            }
            AlertDispatcher::OpsGenie(dispatcher) => {
                dispatcher.process_alerts(feature_alerts, model_name).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scouter::utils::types::{Alert, AlertType, AlertZone, FeatureAlert};
    use std::collections::HashMap;
    #[test]
    fn test_construct_opsgenie_alert_description() {
        let mut features: HashMap<String, FeatureAlert> = HashMap::new();
        features.insert(
            "test_feature_1".to_string(),
            FeatureAlert {
                feature: "test_feature_1".to_string(),
                alerts: vec![Alert {
                    zone: AlertZone::OutOfBounds.to_str(),
                    kind: AlertType::OutOfBounds.to_str(),
                }],
                indices: Default::default(),
            },
        );
        features.insert(
            "test_feature_2".to_string(),
            FeatureAlert {
                feature: "test_feature_2".to_string(),
                alerts: vec![Alert {
                    zone: AlertZone::Zone1.to_str(),
                    kind: AlertType::OutOfBounds.to_str(),
                }],
                indices: Default::default(),
            },
        );
        let alert_description =
            OpsGenieAlertDispatcher::construct_alert_description(&FeatureAlerts { features });
        let expected_alert_description = "Features that have drifted \ntest_feature_1 alerts: \nalert kind Out of bounds -- alert zone: Out of bounds \ntest_feature_2 alerts: \nalert kind Out of bounds -- alert zone: Zone 1 \n".to_string();
        assert_eq!(&alert_description.len(), &expected_alert_description.len());
        assert_eq!(
            &alert_description.contains(
                "test_feature_1 alerts: \nalert kind Out of bounds -- alert zone: Out of bounds"
            ),
            &expected_alert_description.contains(
                "test_feature_1 alerts: \nalert kind Out of bounds -- alert zone: Out of bounds"
            )
        );
        assert_eq!(
            &alert_description.contains(
                "test_feature_2 alerts: \nalert kind Out of bounds -- alert zone: Zone 1"
            ),
            &expected_alert_description.contains(
                "test_feature_2 alerts: \nalert kind Out of bounds -- alert zone: Zone 1"
            )
        );
    }

    #[test]
    fn test_construct_opsgenie_alert_description_empty() {
        let features: HashMap<String, FeatureAlert> = HashMap::new();
        let alert_description =
            OpsGenieAlertDispatcher::construct_alert_description(&FeatureAlerts { features });
        let expected_alert_description = "".to_string();
        assert_eq!(alert_description, expected_alert_description);
    }

    #[test]
    fn test_construct_opsgenie_alert_body() {
        let expected_alert_body = json!(
                {
                    "message": "Model drift detected for test_ml_model",
                    "description": "Features have drifted",
                    "responders":[
                        {"name":"ds-team", "type":"team"}
                    ],
                    "visibleTo":[
                        {"name":"ds-team", "type":"team"}
                    ],
                    "tags": ["Model Drift"],
                    "priority": "P1"
                }
        );
        let alert_body =
            OpsGenieAlertDispatcher::construct_alert_body("Features have drifted", "test_ml_model");
        assert_eq!(alert_body, expected_alert_body);
    }
}