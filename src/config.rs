use serde::{Deserialize, Deserializer};

fn default_listen_addr() -> String {
    "0.0.0.0:7443".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub access_token: String,

    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,
}

fn ensure_trailing_slash<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s.ends_with('/') || s.is_empty() {
        Ok(s)
    } else {
        Ok(format!("{}/", s))
    }
}

fn default_client_id() -> String {
    "awuba-iot".to_string()
}
fn default_discovery_topic_prefix() -> String {
    "homeassistant/".to_string()
}
fn default_state_topic_prefix() -> String {
    "awuba_iot/".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct MqttConfig {
    pub broker_host: String,
    pub broker_port: u16,

    pub broker_username: Option<String>,
    pub broker_password: Option<String>,

    #[serde(default = "default_client_id")]
    pub client_id: String,

    #[serde(default = "default_discovery_topic_prefix", deserialize_with = "ensure_trailing_slash")]
    pub discovery_topic_prefix: String,
    #[serde(default = "default_state_topic_prefix", deserialize_with = "ensure_trailing_slash")]
    pub state_topic_prefix: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub app: AppConfig,
    pub mqtt: MqttConfig,
}
