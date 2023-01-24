use crate::WasmerInstance;
use elrond_exec_service::{
    CompilationOptions, Executor, ExecutorError, Instance, OpcodeCost, ServiceError, VMHooks,
};
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::Arc;

pub struct WasmerExecutor {
    pub data: Rc<WasmerExecutorData>,
}

#[derive(PartialEq, PartialOrd, Eq)]
pub enum WasmerExecutorLogLevel {
    None,
    Debug,
    Trace,
}

impl TryFrom<u64> for WasmerExecutorLogLevel {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            none if none == WasmerExecutorLogLevel::None as u64 => Ok(WasmerExecutorLogLevel::None),
            debug if debug == WasmerExecutorLogLevel::Debug as u64 => {
                Ok(WasmerExecutorLogLevel::Debug)
            }
            trace if trace == WasmerExecutorLogLevel::Trace as u64 => {
                Ok(WasmerExecutorLogLevel::Trace)
            }
            _ => Err("WasmerExecutor undefined log level"),
        }
    }
}

pub struct WasmerExecutorData {
    pub vm_hooks: Rc<Box<dyn VMHooks>>,
    pub opcode_cost: Arc<OpcodeCost>,
    pub log_level: WasmerExecutorLogLevel,
}

impl WasmerExecutorData {
    pub(crate) fn debug(&self, message: &str) {
        if self.log_level >= WasmerExecutorLogLevel::Debug {
            println!("[DEBUG] {}", message);
        }
    }
    #[allow(dead_code)]
    pub(crate) fn trace(&self, message: &str) {
        if self.log_level == WasmerExecutorLogLevel::Trace {
            println!("[TRACE] {}", message);
        }
    }
}

impl Executor for WasmerExecutor {
    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError> {
        self.data.debug("Setting context pointer ...");
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
        self.data.debug("Setting opcode cost ...");
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

    fn set_execution_log_level(&mut self, value: u64) -> Result<(), ExecutorError> {
        if let Some(data_mut) = Rc::get_mut(&mut self.data) {
            let result = WasmerExecutorLogLevel::try_from(value);
            match result {
                Ok(log_level) => {
                    data_mut.log_level = log_level;
                    return Ok(());
                }
                Err(error) => {
                    return Err(Box::new(ServiceError::new(error)));
                }
            }
        }

        Err(Box::new(ServiceError::new(
            "WasmerExecutor log level error",
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
