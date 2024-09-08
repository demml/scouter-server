use anyhow::{Context, Result};
use colored::Colorize;
use scouter::utils::types::FeatureAlerts;
use serde_json::{json, Value};
use std::env;

trait DispatchHelpers {
    fn construct_alert_description(&self, feature_alerts: &FeatureAlerts) -> String {
        let mut alert_description = String::new();
        for (i, (_, feature_alert)) in feature_alerts.features.iter().enumerate() {
            if feature_alert.alerts.is_empty() {
                continue;
            }
            if i == 0 {
                alert_description.push_str("Features that have drifted: \n");
            }

            let feature_name = format!("{:indent$}{}: \n", "", &feature_alert.feature, indent = 4)
                .truecolor(245, 77, 85);

            // can't use push_str when adding colorized strings
            alert_description = format!("{}{}", alert_description, feature_name);
            feature_alert.alerts.iter().for_each(|alert| {
                let kind = format!("{:indent$}Kind: {}\n", "", &alert.kind, indent = 8)
                    .truecolor(249, 179, 93);
                let zone = format!("{:indent$}Zone: {}\n", "", &alert.zone, indent = 8)
                    .truecolor(249, 179, 93);
                alert_description = format!("{}{}{}", alert_description, kind, zone);
            });
        }
        alert_description
    }
}
pub trait Dispatch {
    fn process_alerts(
        &self,
        feature_alerts: &FeatureAlerts,
        repository: &str,
        name: &str,
        version: &str,
    ) -> impl futures::Future<Output = Result<()>>;
}
pub trait HttpAlertWrapper {
    fn api_url(&self) -> &str;
    fn header_auth_value(&self) -> &str;
    fn construct_alert_body(
        &self,
        alert_description: &str,
        repository: &str,
        name: &str,
        version: &str,
    ) -> Value;
}

#[derive(Debug)]
pub struct OpsGenieAlerter {
    header_auth_value: String,
    api_url: String,
}

impl Default for OpsGenieAlerter {
    fn default() -> Self {
        Self {
            header_auth_value: format!(
                "GenieKey {}",
                env::var("OPSGENIE_API_KEY").unwrap_or("api_key".to_string())
            ),
            api_url: format!(
                "{}/alerts",
                env::var("OPSGENIE_API_URL").unwrap_or("api_url".to_string())
            ),
        }
    }
}

impl HttpAlertWrapper for OpsGenieAlerter {
    fn api_url(&self) -> &str {
        &self.api_url
    }

    fn header_auth_value(&self) -> &str {
        &self.header_auth_value
    }

    fn construct_alert_body(
        &self,
        alert_description: &str,
        repository: &str,
        name: &str,
        version: &str,
    ) -> Value {
        json!(
                {
                    "message": format!("Model drift detected for {}/{}/{}", repository, name, version),
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
}
impl DispatchHelpers for OpsGenieAlerter {}
#[derive(Debug)]
pub struct SlackAlerter {
    header_auth_value: String,
    api_url: String,
}

impl Default for SlackAlerter {
    fn default() -> Self {
        Self {
            header_auth_value: format!(
                "Bearer {}",
                env::var("SLACK_BOT_TOKEN").unwrap_or("slack_token".to_string())
            ),
            api_url: format!(
                "{}/chat.postMessage",
                env::var("SLACK_API_URL").unwrap_or("slack_api_url".to_string())
            ),
        }
    }
}

impl HttpAlertWrapper for SlackAlerter {
    fn api_url(&self) -> &str {
        &self.api_url
    }

    fn header_auth_value(&self) -> &str {
        &self.header_auth_value
    }

    fn construct_alert_body(
        &self,
        alert_description: &str,
        repository: &str,
        name: &str,
        version: &str,
    ) -> Value {
        json!({
            "channel": "bot-test",
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": format!(":red_circle: Model drift detected for {}/{}/{} :red_circle:", repository, name, version),
                        "emoji": true
                    }
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": alert_description
                    },
                    "accessory": {
                        "type": "image",
                        "image_url": "https://www.shutterstock.com/shutterstock/photos/2196561307/display_1500/stock-vector--d-vector-yellow-warning-sign-with-exclamation-mark-concept-eps-vector-2196561307.jpg",
                        "alt_text": "Alert Symbol"
                    }
                }
            ]
        })
    }
}

impl DispatchHelpers for SlackAlerter {
    fn construct_alert_description(&self, feature_alerts: &FeatureAlerts) -> String {
        let mut alert_description = String::new();
        for (_, feature_alert) in feature_alerts.features.iter() {
            if feature_alert.alerts.is_empty() {
                continue;
            }
            alert_description.push_str(&format!("*{}* \n", &feature_alert.feature));
            feature_alert.alerts.iter().for_each(|alert| {
                alert_description.push_str(&format!("{} in {} \n", &alert.kind, &alert.zone))
            });
        }
        alert_description
    }
}

#[derive(Debug)]
pub struct HttpAlertDispatcher<T: HttpAlertWrapper> {
    http_client: reqwest::Client,
    alerter: T,
}

impl<T: HttpAlertWrapper> HttpAlertDispatcher<T> {
    pub fn new(alerter: T) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            alerter,
        }
    }

    async fn send_alerts(&self, body: Value) -> Result<()> {
        self.http_client
            .post(self.alerter.api_url())
            .header("Authorization", self.alerter.header_auth_value())
            .json(&body)
            .send()
            .await
            .with_context(|| "Error posting alert to web client")?;
        Ok(())
    }
}

impl<T: HttpAlertWrapper + DispatchHelpers> Dispatch for HttpAlertDispatcher<T> {
    async fn process_alerts(
        &self,
        feature_alerts: &FeatureAlerts,
        repository: &str,
        name: &str,
        version: &str,
    ) -> Result<()> {
        let alert_description = self.alerter.construct_alert_description(feature_alerts);

        if !alert_description.is_empty() {
            let alert_body =
                self.alerter
                    .construct_alert_body(&alert_description, repository, name, version);
            self.send_alerts(alert_body)
                .await
                .with_context(|| "Error sending alerts")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ConsoleAlertDispatcher;

impl Dispatch for ConsoleAlertDispatcher {
    async fn process_alerts(
        &self,
        feature_alerts: &FeatureAlerts,
        repository: &str,
        name: &str,
        version: &str,
    ) -> Result<()> {
        let alert_description = self.construct_alert_description(feature_alerts);

        if !alert_description.is_empty() {
            let msg1 = "Drift detected for".truecolor(245, 77, 85);
            let msg2 = format!("{}/{}/{}!", repository, name, version).truecolor(249, 179, 93);
            let mut body = format!("\n{} {} \n", msg1, msg2);
            body.push_str(&alert_description);

            println!("{}", body);
        }
        Ok(())
    }
}

impl DispatchHelpers for ConsoleAlertDispatcher {}

#[derive(Debug)]
pub enum AlertDispatcher {
    Console(ConsoleAlertDispatcher),
    OpsGenie(HttpAlertDispatcher<OpsGenieAlerter>),
    Slack(HttpAlertDispatcher<SlackAlerter>),
}

impl AlertDispatcher {
    // process alerts can be called asynchronously
    pub async fn process_alerts(
        &self,
        feature_alerts: &FeatureAlerts,
        repository: &str,
        name: &str,
        version: &str,
    ) -> Result<()> {
        match self {
            AlertDispatcher::Console(dispatcher) => dispatcher
                .process_alerts(feature_alerts, repository, name, version)
                .await
                .with_context(|| "Error processing alerts"),
            AlertDispatcher::OpsGenie(dispatcher) => dispatcher
                .process_alerts(feature_alerts, repository, name, version)
                .await
                .with_context(|| "Error processing alerts"),
            AlertDispatcher::Slack(dispatcher) => dispatcher
                .process_alerts(feature_alerts, repository, name, version)
                .await
                .with_context(|| "Error processing alerts"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scouter::utils::types::{Alert, AlertType, AlertZone, FeatureAlert};
    use std::collections::HashMap;
    use std::env;

    fn test_features_hashmap() -> HashMap<String, FeatureAlert> {
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
        features
    }
    #[test]
    fn test_construct_opsgenie_alert_description() {
        let features = test_features_hashmap();
        let alerter = OpsGenieAlerter::default();
        let alert_description = alerter.construct_alert_description(&FeatureAlerts { features });
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
        let alerter = OpsGenieAlerter::default();
        let alert_description = alerter.construct_alert_description(&FeatureAlerts { features });
        let expected_alert_description = "".to_string();
        assert_eq!(alert_description, expected_alert_description);
    }

    #[tokio::test]
    async fn test_construct_opsgenie_alert_body() {
        // set env variables
        let download_server = mockito::Server::new_async().await;
        let url = download_server.url();

        // set env variables
        unsafe {
            env::set_var("OPSGENIE_API_URL", url);
            env::set_var("OPSGENIE_API_KEY", "api_key");
        }
        let expected_alert_body = json!(
                {
                    "message": "Model drift detected for test_repo/test_ml_model/1.0.0",
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
        let alerter = OpsGenieAlerter::default();
        let alert_body = alerter.construct_alert_body(
            "Features have drifted",
            "test_repo",
            "test_ml_model",
            "1.0.0",
        );
        assert_eq!(alert_body, expected_alert_body);
        unsafe {
            env::remove_var("OPSGENIE_API_URL");
            env::remove_var("OPSGENIE_API_KEY");
        }
    }

    #[tokio::test]
    async fn test_send_opsgenie_alerts() {
        let mut download_server = mockito::Server::new_async().await;
        let url = download_server.url();

        // set env variables
        unsafe {
            env::set_var("OPSGENIE_API_URL", url);
            env::set_var("OPSGENIE_API_KEY", "api_key");
        }

        let mock_get_path = download_server
            .mock("Post", "/alerts")
            .with_status(201)
            .create();

        let features = test_features_hashmap();

        let dispatcher =
            AlertDispatcher::OpsGenie(HttpAlertDispatcher::new(OpsGenieAlerter::default()));
        let _ = dispatcher
            .process_alerts(
                &FeatureAlerts { features },
                "test_repo",
                "test_ml_model",
                "1.0.0",
            )
            .await;

        mock_get_path.assert();

        unsafe {
            env::remove_var("OPSGENIE_API_URL");
            env::remove_var("OPSGENIE_API_KEY");
        }
    }

    #[tokio::test]
    async fn test_send_console_alerts() {
        let features = test_features_hashmap();
        let dispatcher = AlertDispatcher::Console(ConsoleAlertDispatcher);
        let result = dispatcher
            .process_alerts(
                &FeatureAlerts { features },
                "test_repo",
                "test_ml_model",
                "1.0.0",
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_slack_alerts() {
        let mut download_server = mockito::Server::new_async().await;
        let url = download_server.url();

        // set env variables
        unsafe {
            env::set_var("SLACK_API_URL", url);
            env::set_var("SLACK_BOT_TOKEN", "bot_token");
        }

        let mock_get_path = download_server
            .mock("Post", "/chat.postMessage")
            .with_status(201)
            .create();

        let features = test_features_hashmap();

        let dispatcher = AlertDispatcher::Slack(HttpAlertDispatcher::new(SlackAlerter::default()));
        let _ = dispatcher
            .process_alerts(
                &FeatureAlerts { features },
                "test_repo",
                "test_ml_model",
                "1.0.0",
            )
            .await;

        mock_get_path.assert();

        unsafe {
            env::remove_var("SLACK_API_URL");
            env::remove_var("SLACK_BOT_TOKEN");
        }
    }

    #[tokio::test]
    async fn test_construct_slack_alert_body() {
        // set env variables
        let download_server = mockito::Server::new_async().await;
        let url = download_server.url();

        unsafe {
            env::set_var("SLACK_API_URL", url);
            env::set_var("SLACK_BOT_TOKEN", "bot_token");
        }
        let expected_alert_body = json!({
            "channel": "bot-test",
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": ":red_circle: Model drift detected for test_repo/test_ml_model/1.0.0 :red_circle:",
                        "emoji": true
                    }
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": "*Features have drifted*"
                    },
                    "accessory": {
                        "type": "image",
                        "image_url": "https://www.shutterstock.com/shutterstock/photos/2196561307/display_1500/stock-vector--d-vector-yellow-warning-sign-with-exclamation-mark-concept-eps-vector-2196561307.jpg",
                        "alt_text": "Alert Symbol"
                    }
                }
            ]
        });
        let alerter = SlackAlerter::default();
        let alert_body = alerter.construct_alert_body(
            "*Features have drifted*",
            "test_repo",
            "test_ml_model",
            "1.0.0",
        );
        assert_eq!(alert_body, expected_alert_body);
        unsafe {
            env::remove_var("SLACK_API_URL");
            env::remove_var("SLACK_BOT_TOKEN");
        }
    }
}
