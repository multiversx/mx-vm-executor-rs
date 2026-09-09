use crate::{CompilationOptionsLegacy, ExecutorError, InstanceLegacy, OpcodeConfig};

use std::ffi::c_void;

pub trait ExecutorLegacy {
    /// Sets the data that can be hold by an instance context.
    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError>;

    /// Sets the opcode version and costs for the given executor.
    fn set_opcode_config(&mut self, opcode_config: OpcodeConfig) -> Result<(), ExecutorError>;

    /// Creates a new VM executor instance.
    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptionsLegacy,
    ) -> Result<Box<dyn InstanceLegacy>, ExecutorError>;

    /// Creates a new VM executor instance from cache.
    fn new_instance_from_cache(
        &self,
        cache_bytes: &[u8],
        compilation_options: &CompilationOptionsLegacy,
    ) -> Result<Box<dyn InstanceLegacy>, ExecutorError>;
}
