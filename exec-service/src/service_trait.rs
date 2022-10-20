use crate::{Executor, VMHooks};

pub type ExecutorError = Box<dyn std::error::Error>;

pub trait ExecutorLastError {
    fn update_last_error_str(&mut self, err_str: String);

    fn get_last_error_string(&self) -> String;
}

pub trait ExecutorService: ExecutorLastError {
    fn push_execution_info(&mut self, line: &str);

    fn get_execution_info(&self) -> String;

    fn clear_execution_info(&mut self);

    fn new_executor(
        &self,
        vm_hooks_builder: Box<dyn VMHooks>,
    ) -> Result<Box<dyn Executor>, ExecutorError>;
}
