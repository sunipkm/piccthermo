#[derive(Debug, Clone)]
pub enum Measurement {
    Temperature(Vec<(u32, f32)>),
    Humidity(Vec<(u32, f32)>),
}

impl Measurement {
    pub fn to_le_bytes(&self) -> Vec<u8> {
        match self {
            Measurement::Temperature(data) => {
                let mut bytes = Vec::with_capacity(16 * data.len()); // 4 bytes for u32 id, 4 bytes for f32 value
                for (id, temp) in data {
                    bytes.extend_from_slice(b"CHRIS,T,"); // Magic number for identification
                    bytes.extend_from_slice(&id.to_le_bytes());
                    bytes.extend_from_slice(&temp.to_le_bytes());
                }
                bytes
            }
            Measurement::Humidity(data) => {
                let mut bytes = Vec::with_capacity(16 * data.len()); // 4 bytes for u32 id, 4 bytes for f32 value
                for (id, temp) in data {
                    bytes.extend_from_slice(b"CHRIS,H,"); // Magic number for identification
                    bytes.extend_from_slice(&id.to_le_bytes());
                    bytes.extend_from_slice(&temp.to_le_bytes());
                }
                bytes
            }
        }
    }
}
