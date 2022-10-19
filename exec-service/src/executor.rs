use crate::{CompilationOptions, ExecutorError, ServiceInstance};

pub trait Executor {
    fn new_instance(
        &self,
        bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn ServiceInstance>, ExecutorError>;
}
