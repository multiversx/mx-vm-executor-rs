use crate::executor_interface::{
    CompilationOptionsLegacy, ExecutorError, ExecutorLegacy, InstanceLegacy, ServiceError,
    VMHooksLegacy,
};
use crate::WasmerInstance;
use log::trace;
use multiversx_chain_vm_executor::OpcodeConfig;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use wasmer_vm::platform_init;

pub fn force_sighandler_reinstall() {
    unsafe {
        platform_init();
    }
}

pub struct WasmerExecutorData {
    vm_hooks: Rc<dyn VMHooksLegacy>,
    opcode_config: Arc<Mutex<OpcodeConfig>>,
}

impl WasmerExecutorData {
    pub fn new(vm_hooks: Box<dyn VMHooksLegacy>) -> Self {
        Self {
            vm_hooks: Rc::from(vm_hooks),
            opcode_config: Arc::new(Mutex::new(OpcodeConfig::default())),
        }
    }

    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError> {
        if let Some(vm_hooks) = Rc::get_mut(&mut self.vm_hooks) {
            vm_hooks.set_vm_hooks_ptr(vm_hooks_ptr);
            Ok(())
        } else {
            Err(Box::new(ServiceError::new(
                "WasmerExecutor already set vmhooks, further configuration not allowed",
            )))
        }
    }

    pub fn set_opcode_config(&mut self, opcode_config: OpcodeConfig) -> Result<(), ExecutorError> {
        match self.opcode_config.lock() {
            Ok(mut opcode_config_ref) => {
                *opcode_config_ref = opcode_config;
                Ok(())
            }
            Err(err) => {
                trace!("Failed to acquire lock for setting opcode config: {}", err);
                Err(Box::new(ServiceError::new(
                    "Failed to acquire lock for setting opcode config",
                )))
            }
        }
    }
}

pub struct WasmerExecutor {
    data: Rc<RefCell<WasmerExecutorData>>,
}

impl WasmerExecutor {
    pub fn new(vm_hooks: Box<dyn VMHooksLegacy>) -> Self {
        Self {
            data: Rc::new(RefCell::new(WasmerExecutorData::new(vm_hooks))),
        }
    }
}

impl ExecutorLegacy for WasmerExecutor {
    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError> {
        trace!("Setting vmhooks ...");
        self.data.borrow_mut().set_vm_hooks_ptr(vm_hooks_ptr)
    }

    fn set_opcode_config(&mut self, opcode_config: OpcodeConfig) -> Result<(), ExecutorError> {
        trace!("Setting opcode config...");
        self.data.borrow_mut().set_opcode_config(opcode_config)
    }

    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptionsLegacy,
    ) -> Result<Box<dyn InstanceLegacy>, ExecutorError> {
        let data = self.data.borrow();
        let instance = WasmerInstance::try_new_instance(
            data.vm_hooks.clone(),
            data.opcode_config.clone(),
            wasm_bytes,
            compilation_options,
        )?;
        Ok(Box::new(instance))
    }

    fn new_instance_from_cache(
        &self,
        cache_bytes: &[u8],
        compilation_options: &CompilationOptionsLegacy,
    ) -> Result<Box<dyn InstanceLegacy>, ExecutorError> {
        let data = self.data.borrow();
        let instance = WasmerInstance::try_new_instance_from_cache(
            data.vm_hooks.clone(),
            data.opcode_config.clone(),
            cache_bytes,
            compilation_options,
        )?;
        Ok(Box::new(instance))
    }
}
