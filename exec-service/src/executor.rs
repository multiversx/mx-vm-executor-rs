use crate::{CompilationOptions, ExecutorError, Instance, OpcodeCost};

use std::ffi::c_void;

pub trait Executor {
    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError>;

    fn new_instance(
        &self,
        bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError>;

    fn set_opcode_cost(&mut self, opcode_cost: &OpcodeCost) -> Result<(), ExecutorError>;
}
