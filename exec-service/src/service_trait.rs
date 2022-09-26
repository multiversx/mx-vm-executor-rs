use crate::{CompilationOptions, ServiceInstance, WasmerImportData};

pub type ExecutorError = Box<dyn std::error::Error>;

pub trait ExecutorLastError {
    fn update_last_error_str(&mut self, err_str: String);

    fn get_last_error_string(&self) -> String;
}

pub trait ExecutorService: ExecutorLastError {
    fn push_execution_info(&mut self, line: &str);

    fn get_execution_info(&self) -> String;

    fn clear_execution_info(&mut self);

    fn set_imports(&mut self, imports: Vec<WasmerImportData>);

    fn new_instance(
        &self,
        bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn ServiceInstance>, ExecutorError>;
}
