use crate::{CompilationOptions, ExecutorError, Instance, OpcodeCost};

use std::ffi::c_void;

pub trait Executor {
    /// Sets the data that can be hold by an instance context.
    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError>;

    /// Sets the opcode costs for the given executor.
    fn set_opcode_cost(&mut self, opcode_cost: &OpcodeCost) -> Result<(), ExecutorError>;

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
