use crate::BreakpointValueLegacy;

/// Represents the reason the executor itself interrupted execution.
///
/// Unlike [`BreakpointValueLegacy`], this only covers the interruptions raised by the
/// executor's own middlewares. Everything the high-level VM signals travels as a
/// [`crate::VMHooksEarlyExit`] instead.
///
/// The discriminants are the values the metering and opcode control middlewares write into
/// the instance's breakpoint global, so that [`BreakpointValue::as_u64`] and
/// [`TryFrom<u64>`] are inverses of each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointValue {
    /// Lack of a breakpoint
    None = 0,

    /// Stopping due to gas being exhausted.
    OutOfGas = 4,

    /// Stopping due to over-allocation of WASM memory.
    MemoryLimit = 5,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakpoint_value_round_trip() {
        for value in [
            BreakpointValue::None,
            BreakpointValue::OutOfGas,
            BreakpointValue::MemoryLimit,
        ] {
            assert_eq!(BreakpointValue::try_from(value.as_u64()).ok(), Some(value));
        }
    }

    #[test]
    fn breakpoint_value_agrees_with_legacy() {
        for value in [
            BreakpointValue::None,
            BreakpointValue::OutOfGas,
            BreakpointValue::MemoryLimit,
        ] {
            assert_eq!(value.as_u64(), value.to_legacy().as_u64());
        }
    }
}
