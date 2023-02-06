use crate::wasmer_logger as WasmerLogger;
use crate::WasmerInstance;
use log::trace;
use multiversx_vm_executor::{
    CompilationOptions, Executor, ExecutorError, Instance, OpcodeCost, ServiceError, VMHooks,
};
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct WasmerExecutorData {
    pub vm_hooks: Rc<Box<dyn VMHooks>>,
    pub opcode_cost: Arc<Mutex<OpcodeCost>>,
}

impl WasmerExecutorData {
    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError> {
        if let Some(vm_hooks) = Rc::get_mut(&mut self.vm_hooks) {
            vm_hooks.set_vm_hooks_ptr(vm_hooks_ptr);
            return Ok(());
        }

        Err(Box::new(ServiceError::new(
            "WasmerExecutor already set vmhooks, further configuration not allowed",
        )))
    }

    fn set_opcode_cost(&mut self, opcode_cost: &OpcodeCost) -> Result<(), ExecutorError> {
        self.opcode_cost.lock().unwrap().clone_from(opcode_cost);
        Ok(())
    }
}

pub struct WasmerExecutor {
    pub data: Rc<RefCell<WasmerExecutorData>>,
}

impl Executor for WasmerExecutor {
    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError> {
        trace!("Setting vmhooks ...");
        self.data.borrow_mut().set_vm_hooks_ptr(vm_hooks_ptr)
    }

    fn set_opcode_cost(&mut self, opcode_cost: &OpcodeCost) -> Result<(), ExecutorError> {
        trace!("Setting opcode cost...");
        self.data.borrow_mut().set_opcode_cost(opcode_cost)
    }

    fn set_execution_log_level(&mut self, value: u64) -> Result<(), ExecutorError> {
        let result = WasmerLogger::u64_to_log_level(value);
        match result {
            Ok(level) => {
                trace!("Setting execution log level to {level}...");
                log::set_max_level(level);
                Ok(())
            }
            Err(error) => Err(Box::new(ServiceError::new(error))),
        }
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
