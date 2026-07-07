use rumqttc::AsyncClient;

use super::discovery::{self, MqttDevice, SensorConfig};
use crate::{protocols::ProtocolType, utils};

pub async fn publish_thermometer_device(
    client: &AsyncClient,
    state_topic: &str,
    discovery_topic_prefix: &str,
    mac_bytes: &[u8; 6],
    identifier: &str,
    protocol_type: ProtocolType,
) -> Result<(), anyhow::Error> {
    let mac = utils::mac6_to_colon_string(mac_bytes);

    let device = MqttDevice {
        connections: Some(vec![("mac".to_string(), mac.to_string())]),
        identifiers: Some(vec![identifier.to_string()]),
        manufacturer: None,
        model: Some(format!("{}", protocol_type)),
        name: Some(format!("BLE {} {}", protocol_type, mac)),
        sw_version: None,
    };

    let sensors = [
        SensorConfig {
            suffix: "temperature",
            name: "Temperature",
            device_class: Some("temperature"),
            unit: Some("°C"),
            value_template: "{{ value_json.temperature }}",
            state_class: Some("measurement"),
            entity_category: None,
        },
        SensorConfig {
            suffix: "humidity",
            name: "Humidity",
            device_class: Some("humidity"),
            unit: Some("%"),
            value_template: "{{ value_json.humidity }}",
            state_class: Some("measurement"),
            entity_category: None,
        },
        SensorConfig {
            suffix: "battery_pct",
            name: "Battery",
            device_class: Some("battery"),
            unit: Some("%"),
            value_template: "{{ value_json.battery_pct }}",
            state_class: Some("measurement"),
            entity_category: Some("diagnostic"),
        },
        SensorConfig {
            suffix: "battery_mv",
            name: "Battery Voltage",
            device_class: Some("voltage"),
            unit: Some("mV"),
            value_template: "{{ value_json.battery_mv }}",
            state_class: Some("measurement"),
            entity_category: Some("diagnostic"),
        },
        SensorConfig {
            suffix: "rssi",
            name: "RSSI",
            device_class: Some("signal_strength"),
            unit: Some("dBm"),
            value_template: "{{ value_json.rssi }}",
            state_class: Some("measurement"),
            entity_category: Some("diagnostic"),
        },
    ];

    discovery::publish_device_sensors(
        client,
        discovery_topic_prefix,
        state_topic,
        identifier,
        &device,
        sensors.into(),
    )
    .await
}
