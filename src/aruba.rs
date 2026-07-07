use anyhow::Context;
use rumqttc::QoS;
use serde::{Deserialize, Serialize};

use crate::{
    AppState, aruba_proto::telemetry::{self, BleFrameType}, hass::{self, thermometer}, protocols::{RegistryParseError, TemperatureProtocolParserError}, utils,
};

#[derive(Debug, Serialize, Deserialize)]
struct PublishedThermometerReading {
    mac: String,
    temperature: f32,
    humidity: f32,
    battery_pct: u8,
    battery_mv: u16,
    rssi: i32,
    reporter_mac: String,
}

pub async fn handle_aruba_nb(state: &AppState, telem: telemetry::Telemetry) -> anyhow::Result<()> {
    let reporter_mac: [u8; 6] = telem
        .reporter
        .mac()
        .try_into()
        .context("invalid reporter mac")?;

    for frame in telem.ble_data {
        if frame.frame_type() != BleFrameType::AdvInd {
            continue;
        }

        tracing::debug!("BLE {reporter_mac:?} sent {:?}", frame);

        let mac: [u8; 6] = match frame.mac().try_into() {
            Ok(m) => m,
            Err(_) => {
                tracing::warn!("skipping frame with invalid MAC address");
                continue;
            }
        };

        let decoded = match state
            .protocols
            .lock()
            .unwrap()
            .parse_frame(mac, frame.data())
        {
            Ok(data) => data,
            Err(RegistryParseError::UnknownService) => {
                tracing::debug!("UnknownPacket");
                continue;
            }
            Err(RegistryParseError::ProtocolError(
                TemperatureProtocolParserError::DuplicatePacket,
            )) => {
                tracing::debug!("DuplicatePacket");
                continue;
            }
            Err(RegistryParseError::ProtocolError(TemperatureProtocolParserError::AtcInvalid)) => {
                tracing::debug!("AtcInvalid");
                continue;
            }
            Err(e) => {
                tracing::error!("unexpected error while parsing frame {e:?}");
                continue;
            }
        };

        tracing::debug!("got temperature data {:?}", decoded);

        let identifier = hass::hass_identifier(&decoded.mac);
        let state_topic = hass::state_topic(
            state.config.mqtt.state_topic_prefix.clone(),
            identifier.clone(),
        );

        let cache_key = (decoded.protocol, decoded.mac);

        let is_new_device = state
            .published_macs
            .lock()
            .unwrap()
            .insert(cache_key, ())
            .is_none();

        if is_new_device {
            tracing::debug!("new device, publishing discovery");

            if let Err(e) = thermometer::publish_thermometer_device(
                &state.mqtt,
                &state_topic,
                &state.config.mqtt.discovery_topic_prefix,
                &decoded.mac,
                &identifier,
                decoded.protocol,
            )
            .await
            {
                state.published_macs.lock().unwrap().remove(&cache_key);
                return Err(e);
            }
        }

        let sensor_payload = PublishedThermometerReading {
            mac: utils::mac6_to_colon_string(&decoded.mac),
            temperature: decoded.temperature,
            humidity: decoded.humidity,
            battery_pct: decoded.battery_percent,
            battery_mv: decoded.battery_mv,
            rssi: frame.rssi(),
            reporter_mac: utils::mac6_to_colon_string(&reporter_mac),
        };

        let payload_json = serde_json::to_string(&sensor_payload)?;

        state
            .mqtt
            .publish(&state_topic, QoS::AtLeastOnce, false, payload_json)
            .await?;
    }
    Ok(())
}
