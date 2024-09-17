use anyhow::{Context, Result};
use colored::Colorize;
use scouter::utils::types::DriftConfig;
use scouter::utils::types::{AlertDispatchType, FeatureAlerts};
use serde_json::{json, Value};

use std::{collections::HashMap, env};
use tracing::error;

const OPSGENIE_API_URL: &str = "https://api.opsgenie.com/v2/alerts";

trait DispatchHelpers {
    fn construct_alert_description(&self, feature_alerts: &FeatureAlerts) -> String {
        let mut alert_description = String::new();
        for (i, (_, feature_alert)) in feature_alerts.features.iter().enumerate() {
            if feature_alert.alerts.is_empty() {
                continue;
            }
            if i == 0 {
                alert_description.push_str("Drift has been detected for the following features:\n");
            }

            let feature_name = format!("{:indent$}{}: \n", "", &feature_alert.feature, indent = 4);

            // can't use push_str when adding colorized strings
            alert_description = format!("{}{}", alert_description, feature_name);
            feature_alert.alerts.iter().for_each(|alert| {
                let kind = format!("{:indent$}Kind: {}\n", "", &alert.kind, indent = 8);
                let zone = format!("{:indent$}Zone: {}\n", "", &alert.zone, indent = 8);
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
    ) -> impl futures::Future<Output = Result<()>>;
}
pub trait HttpAlertWrapper {
    fn api_url(&self) -> &str;
    fn header_auth_value(&self) -> &str;
    fn construct_alert_body(&self, alert_description: &str) -> Value;
}

#[derive(Debug)]
pub struct OpsGenieAlerter {
    header_auth_value: String,
    api_url: String,
    team_name: Option<String>,
    name: String,
    repository: String,
    version: String,
}

impl OpsGenieAlerter {
    /// Create a new OpsGenieAlerter
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the model
    /// * `repository` - Repository of the model
    /// * `version` - Version of the model
    ///
    pub fn new(name: &str, repository: &str, version: &str) -> Result<Self> {
        let api_key = env::var("OPSGENIE_API_KEY").with_context(|| "OPSGENIE_API_KEY not set");
        let api_url = env::var("OPSGENIE_API_URL").unwrap_or(OPSGENIE_API_URL.to_string());
        let team = env::var("OPSGENIE_TEAM").ok();

        match api_key {
            Err(e) => {
                error!("Failed to create OpsGenieAlerter: {:?}", e);
                Err(anyhow::Error::msg("Failed to create OpsGenieAlerter"))
            }
            Ok(api_key) => Ok(Self {
                header_auth_value: format!("GenieKey {}", api_key),
                api_url,
                team_name: team,
                name: name.to_string(),
                repository: repository.to_string(),
                version: version.to_string(),
            }),
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

    fn construct_alert_body(&self, alert_description: &str) -> Value {
        let mut mapping: HashMap<&str, Value> = HashMap::new();
        mapping.insert(
            "message",
            format!(
                "Model drift detected for {}/{}/{}",
                self.repository, self.name, self.version
            )
            .into(),
        );
        mapping.insert("description", alert_description.to_string().into());

        if self.team_name.is_some() {
            mapping.insert(
                "responders",
                json!([{"name": self.team_name.as_ref().unwrap(), "type": "team"}]),
            );
            mapping.insert(
                "visibleTo",
                json!([{"name": self.team_name.as_ref().unwrap(), "type": "team"}]),
            );
        }

        mapping.insert("tags", json!(["Model Drift", "Scouter"]));
        mapping.insert("priority", "P1".into());

        json!(mapping)
    }
}
impl DispatchHelpers for OpsGenieAlerter {}

#[derive(Debug)]
pub struct SlackAlerter {
    header_auth_value: String,
    api_url: String,
    name: String,
    repository: String,
    version: String,
}

impl SlackAlerter {
    /// Create a new SlackAlerter
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the model
    /// * `repository` - Repository of the model
    /// * `version` - Version of the model
    ///
    pub fn new(name: &str, repository: &str, version: &str) -> Result<Self> {
        let app_token = env::var("SLACK_APP_TOKEN").with_context(|| "SLACK_APP_TOKEN not set");
        let api_url = env::var("SLACK_API_URL").with_context(|| "SLACK_API_URL not set");

        match (app_token, api_url) {
            (Err(e), _) => {
                error!("Failed to create SlackAlerter: {:?}", e);
                Err(anyhow::Error::msg("Failed to create SlackAlerter"))
            }
            (_, Err(e)) => {
                error!("Failed to create SlackAlerter: {:?}", e);
                Err(anyhow::Error::msg("Failed to create SlackAlerter"))
            }
            (Ok(app_token), Ok(api_url)) => Ok(Self {
                header_auth_value: format!("Bearer {}", app_token),
                api_url: format!("{}/chat.postMessage", api_url),
                name: name.to_string(),
                repository: repository.to_string(),
                version: version.to_string(),
            }),
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

    fn construct_alert_body(&self, alert_description: &str) -> Value {
        json!({
            "channel": "scouter-bot",
            "blocks": [
                {
                    "type": "header",
                    "text": {
                      "type": "plain_text",
                      "text": ":rotating_light: Drift Detected :rotating_light:",
                      "emoji": true
                    }
                },
                {
                    "type": "section",
                    "text": {
                      "type": "mrkdwn",
                      "text": format!("*Name*: {} *Repository*: {} *Version*: {}", self.name, self.repository, self.version),
                    }
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": alert_description
                    },

                }
            ]
        })
    }
}

impl DispatchHelpers for SlackAlerter {
    fn construct_alert_description(&self, feature_alerts: &FeatureAlerts) -> String {
        let mut alert_description = String::new();
        for (i, (_, feature_alert)) in feature_alerts.features.iter().enumerate() {
            if feature_alert.alerts.is_empty() {
                continue;
            }
            if i == 0 {
                alert_description.push_str("Drift has been detected for the following features:\n");
            }

            let feature_name = format!("{}: \n", &feature_alert.feature);

            // can't use push_str when adding colorized strings
            alert_description = format!("{}{}", alert_description, feature_name);
            feature_alert.alerts.iter().for_each(|alert| {
                let alert = format!(
                    "{:indent$}{} error in {}\n",
                    "",
                    &alert.kind,
                    &alert.zone,
                    indent = 4,
                );
                alert_description = format!("{}{}", alert_description, alert);
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
        let response = self
            .http_client
            .post(self.alerter.api_url())
            .header("Authorization", self.alerter.header_auth_value())
            .json(&body)
            .send()
            .await
            .with_context(|| "Error posting alert to web client")?;

        if response.status().is_success() {
            Ok(())
        } else {
            let text = response
                .text()
                .await
                .unwrap_or("Failed to parse response".to_string());
            error!("Failed to send alert: {}. Continuing", text);
            Ok(())
        }
    }
}

impl<T: HttpAlertWrapper + DispatchHelpers> Dispatch for HttpAlertDispatcher<T> {
    async fn process_alerts(&self, feature_alerts: &FeatureAlerts) -> Result<()> {
        let alert_description = self.alerter.construct_alert_description(feature_alerts);

        let alert_body = self.alerter.construct_alert_body(&alert_description);

        self.send_alerts(alert_body)
            .await
            .with_context(|| "Error sending alerts")?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct ConsoleAlertDispatcher {
    name: String,
    repository: String,
    version: String,
}

impl ConsoleAlertDispatcher {
    pub fn new(name: &str, repository: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            repository: repository.to_string(),
            version: version.to_string(),
        }
    }
}

impl Dispatch for ConsoleAlertDispatcher {
    async fn process_alerts(&self, feature_alerts: &FeatureAlerts) -> Result<()> {
        let alert_description = self.construct_alert_description(feature_alerts);
        if !alert_description.is_empty() {
            let msg1 = "Drift detected for".truecolor(245, 77, 85);
            let msg2 = format!("{}/{}/{}!", self.repository, self.name, self.version)
                .truecolor(249, 179, 93);
            let mut body = format!("\n{} {} \n", msg1, msg2);
            body.push_str(&alert_description);

            println!("{}", body);
        }
        Ok(())
    }
}

impl DispatchHelpers for ConsoleAlertDispatcher {
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

#[derive(Debug)]
pub enum AlertDispatcher {
    Console(ConsoleAlertDispatcher),
    OpsGenie(HttpAlertDispatcher<OpsGenieAlerter>),
    Slack(HttpAlertDispatcher<SlackAlerter>),
}

impl AlertDispatcher {
    // process alerts can be called asynchronously
    pub async fn process_alerts(&self, feature_alerts: &FeatureAlerts) -> Result<()> {
        match self {
            AlertDispatcher::Console(dispatcher) => dispatcher
                .process_alerts(feature_alerts)
                .await
                .with_context(|| "Error processing alerts"),
            AlertDispatcher::OpsGenie(dispatcher) => dispatcher
                .process_alerts(feature_alerts)
                .await
                .with_context(|| "Error processing alerts"),
            AlertDispatcher::Slack(dispatcher) => dispatcher
                .process_alerts(feature_alerts)
                .await
                .with_context(|| "Error processing alerts"),
        }
    }

    pub fn new(config: &DriftConfig) -> Self {
        let name = config.name.clone();
        let repository = config.repository.clone();
        let version = config.version.clone();
        let dispatch_type = &config.alert_config.alert_dispatch_type;

        let alerter = if let AlertDispatchType::OpsGenie = dispatch_type {
            let _alerter = OpsGenieAlerter::new(&name, &repository, &version);
            match _alerter {
                Ok(_alerter) => Ok(AlertDispatcher::OpsGenie(HttpAlertDispatcher::new(
                    _alerter,
                ))),
                Err(e) => {
                    error!("Failed to create OpsGenieAlerter: {:?}", e);
                    Err(anyhow::Error::msg("Failed to create OpsGenieAlerter"))
                }
            }
        } else if let AlertDispatchType::Slack = dispatch_type {
            let _alerter = SlackAlerter::new(&name, &repository, &version);

            match _alerter {
                Ok(_alerter) => Ok(AlertDispatcher::Slack(HttpAlertDispatcher::new(_alerter))),
                Err(e) => {
                    error!("Failed to create SlackAlerter: {:?}", e);
                    Err(anyhow::Error::msg("Failed to create SlackAlerter"))
                }
            }
        } else {
            Ok(AlertDispatcher::Console(ConsoleAlertDispatcher::new(
                &name,
                &repository,
                &version,
            )))
        };

        match alerter {
            Ok(alerter) => alerter,
            Err(e) => {
                error!(
                    "Failed to create AlertDispatcher: {:?}, Defaulting to Console",
                    e
                );
                AlertDispatcher::Console(ConsoleAlertDispatcher::new(&name, &repository, &version))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scouter::utils::types::{
        Alert, AlertConfig, AlertDispatchType, AlertType, AlertZone, DriftConfig, FeatureAlert,
    };

    use std::collections::BTreeMap;
    use std::env;

    fn test_features_map() -> BTreeMap<String, FeatureAlert> {
        let mut features: BTreeMap<String, FeatureAlert> = BTreeMap::new();

        features.insert(
            "test_feature_1".to_string(),
            FeatureAlert {
                feature: "test_feature_1".to_string(),
                alerts: vec![Alert {
                    zone: AlertZone::Zone4.to_str(),
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
                    kind: AlertType::Consecutive.to_str(),
                }],
                indices: Default::default(),
            },
        );
        features
    }
    #[test]
    fn test_construct_opsgenie_alert_description() {
        unsafe {
            env::set_var("OPSGENIE_API_URL", "api_url");
            env::set_var("OPSGENIE_API_KEY", "api_key");
        }
        let features = test_features_map();
        let alerter = OpsGenieAlerter::new("name", "repository", "1.0.0").unwrap();
        let alert_description = alerter.construct_alert_description(&FeatureAlerts {
            features,
            has_alerts: true,
        });
        let expected_alert_description = "Drift has been detected for the following features:\n    test_feature_2: \n        Kind: Consecutive\n        Zone: Zone 1\n    test_feature_1: \n        Kind: Out of bounds\n        Zone: Zone 4\n".to_string();
        assert_eq!(&alert_description.len(), &expected_alert_description.len());

        unsafe {
            env::remove_var("OPSGENIE_API_URL");
            env::remove_var("OPSGENIE_API_KEY");
        }
    }

    #[test]
    fn test_construct_opsgenie_alert_description_empty() {
        unsafe {
            env::set_var("OPSGENIE_API_URL", "api_url");
            env::set_var("OPSGENIE_API_KEY", "api_key");
        }
        let features: BTreeMap<String, FeatureAlert> = BTreeMap::new();
        let alerter = OpsGenieAlerter::new("name", "repository", "1.0.0").unwrap();
        let alert_description = alerter.construct_alert_description(&FeatureAlerts {
            features,
            has_alerts: true,
        });
        let expected_alert_description = "".to_string();
        assert_eq!(alert_description, expected_alert_description);
        unsafe {
            env::remove_var("OPSGENIE_API_URL");
            env::remove_var("OPSGENIE_API_KEY");
        }
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
            env::set_var("OPSGENIE_TEAM", "ds-team");
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
                    "tags": ["Model Drift", "Scouter"],
                    "priority": "P1"
                }
        );
        let alerter = OpsGenieAlerter::new("test_ml_model", "test_repo", "1.0.0").unwrap();
        let alert_body = alerter.construct_alert_body("Features have drifted");
        assert_eq!(alert_body, expected_alert_body);
        unsafe {
            env::remove_var("OPSGENIE_API_URL");
            env::remove_var("OPSGENIE_API_KEY");
            env::remove_var("OPSGENIE_TEAM");
        }
    }

    #[tokio::test]
    async fn test_send_opsgenie_alerts() {
        let mut download_server = mockito::Server::new_async().await;
        let url = format!("{}/alerts", download_server.url());

        // set env variables
        unsafe {
            env::set_var("OPSGENIE_API_URL", url);
            env::set_var("OPSGENIE_API_KEY", "api_key");
        }

        let mock_get_path = download_server
            .mock("Post", "/alerts")
            .with_status(201)
            .create();

        let features = test_features_map();

        let dispatcher = AlertDispatcher::OpsGenie(HttpAlertDispatcher::new(
            OpsGenieAlerter::new("name", "repository", "1.0.0").unwrap(),
        ));
        let _ = dispatcher
            .process_alerts(&FeatureAlerts {
                features,
                has_alerts: true,
            })
            .await;

        mock_get_path.assert();

        unsafe {
            env::remove_var("OPSGENIE_API_URL");
            env::remove_var("OPSGENIE_API_KEY");
        }
    }

    #[tokio::test]
    async fn test_send_console_alerts() {
        let features = test_features_map();
        let dispatcher =
            AlertDispatcher::Console(ConsoleAlertDispatcher::new("name", "repository", "1.0.0"));
        let result = dispatcher
            .process_alerts(&FeatureAlerts {
                features,
                has_alerts: true,
            })
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
            env::set_var("SLACK_APP_TOKEN", "bot_token");
        }

        let mock_get_path = download_server
            .mock("Post", "/chat.postMessage")
            .with_status(201)
            .create();

        let features = test_features_map();

        let dispatcher = AlertDispatcher::Slack(HttpAlertDispatcher::new(
            SlackAlerter::new("name", "repository", "1.0.0").unwrap(),
        ));
        let _ = dispatcher
            .process_alerts(&FeatureAlerts {
                features,
                has_alerts: true,
            })
            .await;

        mock_get_path.assert();

        unsafe {
            env::remove_var("SLACK_API_URL");
            env::remove_var("SLACK_APP_TOKEN");
        }
    }

    #[tokio::test]
    async fn test_construct_slack_alert_body() {
        // set env variables
        let download_server = mockito::Server::new_async().await;
        let url = download_server.url();

        unsafe {
            env::set_var("SLACK_API_URL", url);
            env::set_var("SLACK_APP_TOKEN", "bot_token");
        }
        let expected_alert_body = json!({
            "channel": "scouter-bot",
            "blocks": [
                {
                    "type": "header",
                    "text": {
                        "type": "plain_text",
                        "text": ":rotating_light: Drift Detected :rotating_light:",
                        "emoji": true
                    }
                },
                {
                    "type": "section",
                    "text": {
                      "type": "mrkdwn",
                      "text": "*Name*: name *Repository*: repository *Version*: 1.0.0",
                    }
                },
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": "*Features have drifted*"
                    },
                }
            ]
        });
        let alerter = SlackAlerter::new("name", "repository", "1.0.0").unwrap();
        let alert_body = alerter.construct_alert_body("*Features have drifted*");
        assert_eq!(alert_body, expected_alert_body);
        unsafe {
            env::remove_var("SLACK_API_URL");
            env::remove_var("SLACK_APP_TOKEN");
        }
    }

    #[test]
    fn test_console_dispatcher_returned_when_env_vars_not_set_opsgenie() {
        unsafe {
            env::remove_var("OPSGENIE_API_KEY");
        }
        let alert_config =
            AlertConfig::new(None, Some(AlertDispatchType::OpsGenie), None, None, None);

        let config = DriftConfig::new(
            Some("name".to_string()),
            Some("repository".to_string()),
            Some("1.0.0".to_string()),
            None,
            None,
            None,
            None,
            Some(alert_config),
            None,
        )
        .unwrap();
        let dispatcher = AlertDispatcher::new(&config);
        assert!(
            matches!(dispatcher, AlertDispatcher::Console(_)),
            "Expected Console Dispatcher"
        );
    }

    #[test]
    fn test_console_dispatcher_returned_when_env_vars_not_set_slack() {
        unsafe {
            env::remove_var("SLACK_API_URL");
            env::remove_var("SLACK_APP_TOKEN");
        }

        let alert_config = AlertConfig::new(None, Some(AlertDispatchType::Slack), None, None, None);
        let config = DriftConfig::new(
            Some("name".to_string()),
            Some("repository".to_string()),
            Some("1.0.0".to_string()),
            None,
            None,
            None,
            None,
            Some(alert_config),
            None,
        )
        .unwrap();

        let dispatcher = AlertDispatcher::new(&config);
        assert!(
            matches!(dispatcher, AlertDispatcher::Console(_)),
            "Expected Console Dispatcher"
        );
    }

    #[test]
    fn test_slack_dispatcher_returned_when_env_vars_set() {
        unsafe {
            env::set_var("SLACK_API_URL", "url");
            env::set_var("SLACK_APP_TOKEN", "bot_token");
        }
        let alert_config = AlertConfig::new(None, Some(AlertDispatchType::Slack), None, None, None);
        let config = DriftConfig::new(
            Some("name".to_string()),
            Some("repository".to_string()),
            Some("1.0.0".to_string()),
            None,
            None,
            None,
            None,
            Some(alert_config),
            None,
        )
        .unwrap();

        let dispatcher = AlertDispatcher::new(&config);

        assert!(
            matches!(dispatcher, AlertDispatcher::Slack(_)),
            "Expected Slack Dispatcher"
        );

        unsafe {
            env::remove_var("SLACK_API_URL");
            env::remove_var("SLACK_APP_TOKEN");
        }
    }
}
