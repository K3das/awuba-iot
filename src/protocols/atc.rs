use hashlink::LruCache;

use crate::protocols::ProtocolType;

use super::{ParsedTemperature, TemperatureProtocolParser, TemperatureProtocolParserError};

pub struct AtcParser {
    last_packets: LruCache<[u8; 6], u8>,
}

impl AtcParser {
    pub fn new() -> Self {
        AtcParser {
            last_packets: LruCache::new(512),
        }
    }
}

impl TemperatureProtocolParser for AtcParser {
    fn uuid(&self) -> uuid::Uuid {
        btsensor::atc::UUID
    }

    fn parse_service_data(
        &mut self,
        _mac: [u8; 6],
        service_data: &[u8],
    ) -> Result<ParsedTemperature, TemperatureProtocolParserError> {
        let reading = btsensor::atc::SensorReading::decode(service_data)
            .ok_or(TemperatureProtocolParserError::AtcInvalid)?;

        let (parsed_reading, counter) = match reading {
            btsensor::atc::SensorReading::Atc {
                mac,
                temperature,
                humidity,
                battery_percent,
                battery_mv,
                packet_counter,
            } => (
                ParsedTemperature {
                    protocol: ProtocolType::Atc1441,
                    mac,
                    temperature: f32::from(temperature) / 100.0,
                    humidity: humidity.into(),
                    battery_percent,
                    battery_mv,
                },
                packet_counter,
            ),
            btsensor::atc::SensorReading::Pvvx {
                mac,
                temperature,
                humidity,
                battery_mv,
                battery_percent,
                counter,
                flags: _,
            } => (
                ParsedTemperature {
                    protocol: ProtocolType::AtcPvvx,
                    mac,
                    temperature: f32::from(temperature) / 100.0,
                    humidity: f32::from(humidity) / 100.0,
                    battery_percent,
                    battery_mv,
                },
                counter,
            ),
        };

        let last_packet_id = self.last_packets.get(&parsed_reading.mac);

        if last_packet_id.is_some_and(|x| *x == counter) {
            Err(TemperatureProtocolParserError::DuplicatePacket)
        } else {
            self.last_packets.insert(parsed_reading.mac, counter);

            Ok(parsed_reading)
        }
    }
}
