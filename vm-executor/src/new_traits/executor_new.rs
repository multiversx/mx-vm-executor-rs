use crate::{CompilationOptions, ExecutorError, Instance};

/// Creates instances, and nothing else.
///
/// An executor is a factory and an instance-type selector. It holds only what is constant
/// across every instance it will ever make: the [`crate::ExecutorRuntime`] behind it, and
/// whatever plumbing the backend itself needs, such as logging. Everything that is execution
/// logic — the opcode costs, the memory limits — arrives per instance, in
/// [`CompilationOptions`].
///
/// There are no setters. A host that needs different compilation options passes different
/// options; a host that needs a different runtime builds a different executor.
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
