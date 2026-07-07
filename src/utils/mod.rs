const BLUETOOTH_BASE_UUID: u128 = 0x00000000_0000_1000_8000_00805f9b34fb;

pub fn uuid16_to_uuid(uuid: u16) -> uuid::Uuid {
    uuid::Uuid::from_u128(BLUETOOTH_BASE_UUID | ((uuid as u128) << 96))
}

pub fn mac6_to_colon_string(mac: &[u8; 6]) -> String {
    mac.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join(":")
}

pub fn mac6_to_string(mac: &[u8; 6]) -> String {
    mac.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<String>>()
        .join("")
}
