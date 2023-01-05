use crate::WasmerInstance;
use mx_vm_executor::{
    CompilationOptions, Executor, ExecutorError, Instance, OpcodeCost, ServiceError, VMHooks,
};
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Arc;

pub struct WasmerExecutor {
    pub data: Rc<WasmerExecutorData>,
}

pub struct WasmerExecutorData {
    pub vm_hooks: Rc<Box<dyn VMHooks>>,
    pub opcode_cost: Arc<OpcodeCost>,
    pub print_execution_info: bool,
}

impl WasmerExecutorData {
    pub(crate) fn print_execution_info(&self, message: &str) {
        if self.print_execution_info {
            println!("{}", message);
        }
    }
}

impl Executor for WasmerExecutor {
    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError> {
        self.data
            .print_execution_info("Setting context pointer ...");
        if let Some(data_mut) = Rc::get_mut(&mut self.data) {
            if let Some(vm_hooks) = Rc::get_mut(&mut data_mut.vm_hooks) {
                vm_hooks.set_vm_hooks_ptr(vm_hooks_ptr);
                return Ok(());
            }
        }

        Err(Box::new(ServiceError::new(
            "WasmerExecutor already has instances, can no longer be configured",
        )))
    }

    fn set_opcode_cost(&mut self, opcode_cost: &OpcodeCost) -> Result<(), ExecutorError> {
        self.data.print_execution_info("Setting opcode cost ...");
        if let Some(data_mut) = Rc::get_mut(&mut self.data) {
            if let Some(opcode_cost_mut) = Arc::get_mut(&mut data_mut.opcode_cost) {
                *opcode_cost_mut = opcode_cost.clone();
                return Ok(());
            }
        }

        Err(Box::new(ServiceError::new(
            "WasmerExecutor opcodes cost configuration error",
        )))
    }

    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        WasmerInstance::try_new_instance(self.data.clone(), wasm_bytes, compilation_options)
    }

    fn new_instance_from_cache(
        &self,
        cache_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        WasmerInstance::try_new_instance_from_cache(
            self.data.clone(),
            cache_bytes,
            compilation_options,
        )
    }
}
