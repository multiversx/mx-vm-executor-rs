use crate::{CompilationOptions, ExecutorError, Instance};

pub trait Executor {
    /// Creates a new VM executor instance.
    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError>;

    /// Creates a new VM executor instance from cache.
    fn new_instance_from_cache(
        &self,
        cache_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError>;
}
