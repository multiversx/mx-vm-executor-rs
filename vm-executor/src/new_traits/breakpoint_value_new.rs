use crate::BreakpointValueLegacy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointValue {
    /// Lack of a breakpoint
    None = 0,

    /// Stopping due to gas being exhausted.
    OutOfGas = 1,

    /// Stopping due to over-allocation of WASM memory.
    MemoryLimit = 2,
}

impl BreakpointValue {
    pub fn as_u64(self) -> u64 {
        self as u64
    }

    pub fn to_legacy(self) -> BreakpointValueLegacy {
        match self {
            BreakpointValue::None => BreakpointValueLegacy::None,
            BreakpointValue::OutOfGas => BreakpointValueLegacy::OutOfGas,
            BreakpointValue::MemoryLimit => BreakpointValueLegacy::MemoryLimit,
        }
    }
}

pub struct UnknownBreakpointValueError;

impl TryFrom<u64> for BreakpointValue {
    type Error = UnknownBreakpointValueError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BreakpointValue::None),
            4 => Ok(BreakpointValue::OutOfGas),
            5 => Ok(BreakpointValue::MemoryLimit),
            _ => Err(UnknownBreakpointValueError),
        }
    }
}
