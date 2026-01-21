#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpcodeVersion {
    V1,
    V2,
}

impl OpcodeVersion {
    pub fn from_i32(value: i32) -> Option<Self> {
        match value {
            0 => Some(OpcodeVersion::V1),
            1 => Some(OpcodeVersion::V2),
            _ => None,
        }
    }
}
