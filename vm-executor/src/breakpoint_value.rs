#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakpointValue {
    /// Lack of a breakpoint
    None = 0,

    /// Failure indicated by the high-level VM (in the VMHooks).
    ExecutionFailed = 1,

    /// Stopping execution due to an async call.
    AsyncCall = 2,

    /// Stopping due to an error signalled by the contract.
    SignalError = 3,

    /// Stopping due to gas being exhausted.
    OutOfGas = 4,

    /// Stopping due to over-allocation of WASM memory.
    MemoryLimit = 5,
}

impl BreakpointValue {
    pub fn as_u64(self) -> u64 {
        self as u64
    }
}

impl TryFrom<u64> for BreakpointValue {
    type Error = String;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BreakpointValue::None),
            1 => Ok(BreakpointValue::ExecutionFailed),
            2 => Ok(BreakpointValue::AsyncCall),
            3 => Ok(BreakpointValue::SignalError),
            4 => Ok(BreakpointValue::OutOfGas),
            5 => Ok(BreakpointValue::MemoryLimit),
            _ => Err("unknown breakpoint".to_string()),
        }
    }
}
