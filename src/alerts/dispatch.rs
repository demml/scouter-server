use crate::sql::postgres::PostgresClient;
use anyhow::{Context, Result};
use scouter::utils::types::{Alert, FeatureAlerts};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;

trait AlertDispatcher {
    async fn process_alerts(&self);
    async fn send_alert(&self, message: &str);
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
            ops_genie_api_url: env::var("OPSGENIE_API_URL")
                .unwrap_or("api_url".to_string()),
            ops_genie_api_key: env::var("OPSGENIE_API_KEY").unwrap_or("api_key".to_string()),
            http_client: reqwest::Client::new(),
        }
    }
}

impl OpsGenieAlertDispatcher {
    pub async fn process_alerts(&self, feature_alerts: FeatureAlerts, model_name: &str) -> Result<()> {
        let mut alert_description = String::new();
        for (i, (_, feature_alert))in feature_alerts.features.iter().enumerate() {
            if feature_alert.alerts.is_empty() {
                continue;
            }
            if i == 0{
                alert_description.push_str("Features that have drifted \n");
            }
            alert_description.push_str(&format!("{} alerts: \n", &feature_alert.feature));
            feature_alert.alerts.iter().for_each(|alert| {
                alert_description.push_str(&format!("alert kind {} -- alert zone: {} \n", &alert.kind, &alert.zone))
            });
        }
        let alert_body = Self::construct_alert_body(&alert_description, model_name);
        self.send_alerts(alert_body).await?;
        Ok(())
    }

    fn construct_alert_body(alert_description: &str, model_name: &str)-> Value {
        json!(
                {
                    "message": format!("Model drift detected for model example {}", model_name),
                    // TODO create unique string for alias
                    "alias": "123abc",
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
            .await?;
        Ok(())
    }
}
