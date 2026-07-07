use crate::utils;

pub mod discovery;
pub mod thermometer;

pub fn hass_identifier(mac: &[u8; 6]) -> String {
    let mac = utils::mac6_to_string(mac);
    format!("awuba_ble_{}", mac)
}

pub fn state_topic(prefix: String, identifier: String) -> String {
    format!("{}{}/state", prefix, identifier)
}
