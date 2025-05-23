use std::{borrow::Cow, fmt};

/// Contains details regarding an early exit triggered by a VM hook.
///
/// It doesn't have to be an error, for instance in the case of legacy async calls
/// execution is intentionally halted early.
#[derive(Debug, Clone)]
pub struct VMHooksEarlyExit {
    pub code: u64,
    pub message: Cow<'static, str>,
}

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
