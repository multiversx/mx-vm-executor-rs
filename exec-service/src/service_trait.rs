use crate::{Executor, VMHooks};

pub type ExecutorError = Box<dyn std::error::Error>;

pub trait ExecutorLastError {
    fn update_last_error_str(&mut self, err_str: String);

    fn get_last_error_string(&self) -> String;
}

pub trait ExecutorService: ExecutorLastError {
    fn new_executor(
        &self,
        vm_hooks_builder: Box<dyn VMHooks>,
    ) -> Result<Box<dyn Executor>, ExecutorError>;
}
