use std::{borrow::Cow, fmt};

use crate::BreakpointValueLegacy;

/// Contains details regarding an early exit triggered by a VM hook.
///
/// It doesn't have to be an error, for instance in the case of legacy async calls
/// execution is intentionally halted early.
#[derive(Debug, Clone)]
pub struct VMHooksEarlyExit {
    pub code: u64,
    pub message: Cow<'static, str>,
}

/// Legacy async call. Code 0 is reserved for it on both sides of the ABI, which is why the
/// C hook result carries a separate "no early exit" sentinel instead of using 0 for it.
const CODE_ASYNC_CALL: u64 = 0;

/// An error signalled by the contract itself.
const CODE_USER_ERROR: u64 = 4;

/// Gas exhausted.
const CODE_OUT_OF_GAS: u64 = 5;

impl VMHooksEarlyExit {
    pub fn new(code: u64) -> Self {
        VMHooksEarlyExit {
            code,
            message: Cow::Borrowed(""),
        }
    }

    pub fn with_const_message(mut self, message: &'static str) -> Self {
        self.message = Cow::Borrowed(message);
        self
    }

    pub fn with_message(mut self, message: String) -> Self {
        self.message = Cow::Owned(message);
        self
    }

    /// Which breakpoint halts a Wasmer 2.2 instance for this exit.
    ///
    /// **Only the legacy backend asks.** Its hooks return plain values, with no channel for
    /// an early exit, so the instance has to be stopped by tripping its breakpoint global.
    /// Wasmer 7 hooks return the early exit directly and never come here.
    ///
    /// The codes are the high-level VM's own return codes, and for a while this looked like
    /// something only a host could decide. It is not: the Go VM and the Rust VM use the same
    /// numbering and reach the same three answers — compare `applyEarlyExit` in the Go VM's
    /// `vmhost/vmhooks/earlyExit.go`, which is this same switch. So it lives on the exit
    /// itself, and no host implements anything.
    ///
    /// Everything else is a failure, which is also what an unrecognised code has to be: the
    /// instance must stop either way, and stopping it as a failure is the safe reading.
    pub fn breakpoint_legacy(&self) -> BreakpointValueLegacy {
        match self.code {
            CODE_ASYNC_CALL => BreakpointValueLegacy::AsyncCall,
            CODE_USER_ERROR => BreakpointValueLegacy::SignalError,
            CODE_OUT_OF_GAS => BreakpointValueLegacy::OutOfGas,
            _ => BreakpointValueLegacy::ExecutionFailed,
        }
    }
}

impl fmt::Display for VMHooksEarlyExit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("VMHooksEarlyExit")
            .field("code", &self.code)
            .field("message", &self.message)
            .finish()
    }
}

impl std::error::Error for VMHooksEarlyExit {}

#[cfg(test)]
mod tests {
    use super::*;

    /// The mapping has no runtime observer yet: the Rust VM reads the stashed exit before it
    /// reads the breakpoint, and the Go VM's hooks set the breakpoint themselves. It becomes
    /// observable the moment the Go VM runs Wasmer 2.2 through the new C API, so it is pinned
    /// here instead of being left to a test that cannot fail.
    #[test]
    fn breakpoint_legacy_matches_both_high_level_vms() {
        let bp = |code: u64| VMHooksEarlyExit::new(code).breakpoint_legacy();

        // Go: BreakpointAsyncCall, set directly by the async hook. Rust VM: code 0.
        assert_eq!(bp(CODE_ASYNC_CALL), BreakpointValueLegacy::AsyncCall);

        // Go: SignalUserError -> BreakpointSignalError.
        assert_eq!(bp(CODE_USER_ERROR), BreakpointValueLegacy::SignalError);

        // Go: SetRuntimeBreakpointValue(BreakpointOutOfGas).
        assert_eq!(bp(CODE_OUT_OF_GAS), BreakpointValueLegacy::OutOfGas);

        // Go: FailExecution -> BreakpointExecutionFailed. 10 is vmcommon.ExecutionFailed,
        // which the Go VM sends as the default for any hook error that does not describe
        // its own exit.
        assert_eq!(bp(10), BreakpointValueLegacy::ExecutionFailed);

        // A code neither VM uses still has to stop the instance.
        assert_eq!(bp(u64::MAX), BreakpointValueLegacy::ExecutionFailed);
    }
}
