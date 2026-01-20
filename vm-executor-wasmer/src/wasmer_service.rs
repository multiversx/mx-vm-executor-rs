use crate::executor_interface::{
    ExecutorError, ExecutorLastError, ExecutorLegacy, ExecutorService, VMHooksLegacy,
};
use log::trace;

use crate::WasmerExecutor;

#[derive(Default)]
pub struct BasicExecutorService {
    pub last_error: String,
}

impl BasicExecutorService {
    pub fn new() -> Self {
        Self {
            last_error: String::new(),
        }
    }
}

impl ExecutorLastError for BasicExecutorService {
    fn update_last_error_str(&mut self, err_str: String) {
        self.last_error = err_str;
    }

    fn get_last_error_string(&self) -> String {
        self.last_error.clone()
    }
}

impl ExecutorService for BasicExecutorService {
    fn new_executor(
        &self,
        vm_hooks_builder: Box<dyn VMHooksLegacy>,
    ) -> Result<Box<dyn ExecutorLegacy>, ExecutorError> {
        trace!("Initializing WasmerExecutor ...");
        Ok(Box::new(WasmerExecutor::new(vm_hooks_builder)))
    }
}
