#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpcodeCheckUsed {
    MemoryCopy,
    MemoryFill,
}

impl OpcodeCheckUsed {
    pub fn from_value(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::MemoryCopy),
            1 => Some(Self::MemoryFill),
            _ => None,
        }
    }

    pub fn as_u32(self) -> u32 {
        match self {
            Self::MemoryCopy => 0,
            Self::MemoryFill => 1,
        }
    }
}
