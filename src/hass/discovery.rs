use rumqttc::{AsyncClient, QoS};
use serde::{Deserialize, Serialize};

/// Information about the device this sensor is a part of to tie it into the device registry. Only works when unique_id is set. At least one of identifiers or connections must be present to identify the device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttDevice {
    /// A list of connections of the device to the outside world as a list of tuples [connection_type, connection_identifier] . For example the MAC address of a network interface: "connections": [["mac", "02:5b:26:a8:dc:12"]] .
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connections: Option<Vec<(String, String)>>,

    /// A list of IDs that uniquely identify the device. For example a serial number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifiers: Option<Vec<String>>,

    /// The manufacturer of the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer: Option<String>,

    /// The model of the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// The name of the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The firmware version of the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sw_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttSensorDiscovery {
    /// Information about the device this sensor is a part of to tie it into the device registry. Only works when unique_id is set. At least one of identifiers or connections must be present to identify the device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device: Option<MqttDevice>,

    /// The type/class of the sensor to set the icon in the frontend. The device_class can be null.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_class: Option<String>,

    /// The name of the MQTT sensor. Can be set to null if only the device name is relevant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The MQTT topic subscribed to receive sensor values.
    pub state_topic: String,

    /// An ID that uniquely identifies this sensor. If two sensors have the same unique ID, Home Assistant will raise an exception.
    pub unique_id: String,

    /// Defines the units of measurement of the sensor, if any. The unit_of_measurement can be null.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_of_measurement: Option<String>,

    /// Defines a template to extract the value. If the template throws an error, the current state will be used instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_template: Option<String>,

    /// The category of the entity. When set, the entity category must be diagnostic for sensors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_category: Option<String>,

    /// The state_class of the sensor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_class: Option<String>,
}

pub struct SensorConfig {
    pub suffix: &'static str,
    pub name: &'static str,
    pub device_class: Option<&'static str>,
    pub unit: Option<&'static str>,
    pub value_template: &'static str,
    pub state_class: Option<&'static str>,
    pub entity_category: Option<&'static str>,
}

pub async fn publish_device_sensors(
    client: &AsyncClient,
    discovery_topic_prefix: &str,
    state_topic: &str,
    identifier: &str,
    device: &MqttDevice,
    sensors: Vec<SensorConfig>,
) -> Result<(), anyhow::Error> {
    for sensor in sensors {
        let unique_id = format!("{}_{}", identifier, sensor.suffix);

        let discovery_topic = format!(
            "{}sensor/{}/{}/config",
            discovery_topic_prefix, identifier, sensor.suffix
        );

        let discovery_payload = MqttSensorDiscovery {
            device: Some(device.clone()),
            device_class: sensor.device_class.map(String::from),
            name: Some(sensor.name.to_string()),
            state_topic: state_topic.to_string(),
            unique_id,
            unit_of_measurement: sensor.unit.map(String::from),
            value_template: Some(sensor.value_template.to_string()),
            entity_category: sensor.entity_category.map(String::from),
            state_class: sensor.state_class.map(String::from),
        };

        let payload_json = serde_json::to_string(&discovery_payload)?;

        client
            .publish(&discovery_topic, QoS::AtLeastOnce, true, payload_json)
            .await?;
    }

    Ok(())
}
