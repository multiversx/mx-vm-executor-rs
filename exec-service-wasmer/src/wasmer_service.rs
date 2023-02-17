use log::trace;
use log::LevelFilter;
use multiversx_vm_executor::{
    Executor, ExecutorError, ExecutorLastError, ExecutorService, VMHooks,
};

use crate::wasmer_logger as WasmerLogger;
use crate::WasmerExecutor;

#[derive(Default)]
pub struct BasicExecutorService {
    pub last_error: String,
}

impl BasicExecutorService {
    pub fn new() -> Self {
        Self::init();
        Self {
            last_error: String::new(),
        }
    }

    fn init() {
        // Initialize the logger only once
        // WasmerLogger::init(LevelFilter::Off);
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
        vm_hooks_builder: Box<dyn VMHooks>,
    ) -> Result<Box<dyn Executor>, ExecutorError> {
        trace!("Initializing WasmerExecutor ...");
        Ok(Box::new(WasmerExecutor::new(vm_hooks_builder)))
    }
}
