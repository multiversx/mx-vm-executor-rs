use crate::{Executor, VMHooks};

pub type ExecutorError = Box<dyn std::error::Error>;

pub trait ExecutorLastError {
    /// Updates the last known error.
    fn update_last_error_str(&mut self, err_str: String);

    /// Returns the last known error.
    fn get_last_error_string(&self) -> String;
}

pub trait ExecutorService: ExecutorLastError {
    /// Creates a new VM executor.
    fn new_executor(
        &self,
        vm_hooks_builder: Box<dyn VMHooks>,
    ) -> Result<Box<dyn Executor>, ExecutorError>;
}
