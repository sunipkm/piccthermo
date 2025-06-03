
#[derive(Debug, Clone)]
pub enum Measurement {
    Temperature(Vec<(u32, f32)>),
    Humidity(Vec<(u32, f32)>),
}

impl Measurement {
    pub fn to_le_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16); // 4 bytes for u32 id, 4 bytes for f32 value
        bytes.extend_from_slice(b"CHRISRCV"); // Magic number for identification
        match self {
            Measurement::Temperature(data) => {
                bytes.push(b'T');
                for (id, temp) in data {
                    bytes.extend_from_slice(&id.to_le_bytes());
                    bytes.extend_from_slice(&temp.to_le_bytes());
                }
                bytes
            }
            Measurement::Humidity(data) => {
                bytes.push(b'H');
                for (id, hum) in data {
                    bytes.extend_from_slice(&id.to_le_bytes());
                    bytes.extend_from_slice(&hum.to_le_bytes());
                }
                bytes
            }
        }
    }
}