use core::fmt;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use trouble_host::advertise::AdStructure;

pub mod atc;
use atc::AtcParser;

use crate::utils::uuid16_to_uuid;

#[derive(Serialize, Deserialize, Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolType {
    Atc1441,
    AtcPvvx,
}

impl fmt::Display for ProtocolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.serialize(f)
    }
}

#[derive(Debug)]
pub struct ParsedTemperature {
    pub protocol: ProtocolType,
    pub mac: [u8; 6],
    pub temperature: f32,
    pub humidity: f32,
    pub battery_percent: u8,
    pub battery_mv: u16,
}

#[derive(Error, Debug)]
pub enum TemperatureProtocolParserError {
    #[error("duplicate packet")]
    DuplicatePacket,
    #[error("invalid atc payload")]
    AtcInvalid,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub trait TemperatureProtocolParser: Send + Sync {
    fn uuid(&self) -> uuid::Uuid;

    fn parse_service_data(
        &mut self,
        mac: [u8; 6],
        service_data: &[u8],
    ) -> Result<ParsedTemperature, TemperatureProtocolParserError>;
}

pub struct ParserRegistry {
    parsers: HashMap<uuid::Uuid, Box<dyn TemperatureProtocolParser>>,
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Error, Debug)]
pub enum RegistryParseError {
    #[error("invalid frame")]
    InvalidFrame,
    #[error("couldn't parse advertisement payload: {0}")]
    ProtocolError(#[from] TemperatureProtocolParserError),
    #[error("unknown service")]
    UnknownService,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self {
            parsers: HashMap::new(),
        }
    }

    pub fn with_parser<P: TemperatureProtocolParser + 'static>(mut self, parser: P) -> Self {
        self.parsers.insert(parser.uuid(), Box::new(parser));
        self
    }

    pub fn default_handlers() -> Self {
        Self::new().with_parser(AtcParser::new())
    }

    pub fn parse(
        &mut self,
        uuid: &uuid::Uuid,
        mac: [u8; 6],
        service_data: &[u8],
    ) -> Result<ParsedTemperature, RegistryParseError> {
        let parser = self
            .parsers
            .get_mut(uuid)
            .ok_or(RegistryParseError::UnknownService)?;

        parser
            .parse_service_data(mac, service_data)
            .map_err(|e| e.into())
    }

    pub fn parse_frame(
        &mut self,
        mac: [u8; 6],
        payload: &[u8],
    ) -> Result<ParsedTemperature, RegistryParseError> {
        for ele in AdStructure::decode(payload) {
            let ad = ele.map_err(|_| RegistryParseError::InvalidFrame)?;
            if let AdStructure::ServiceData16 { uuid, data } = ad {
                let service_uuid = uuid16_to_uuid(u16::from_le_bytes(uuid));

                return self.parse(&service_uuid, mac, data);
            }
        }

        Err(RegistryParseError::InvalidFrame)
    }
}
