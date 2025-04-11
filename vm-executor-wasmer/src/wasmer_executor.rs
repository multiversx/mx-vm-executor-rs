use crate::WasmerInstance;
use log::trace;
use multiversx_chain_vm_executor::{
    CompilationOptions, ExecutorError, ExecutorFull, InstanceFull, OpcodeCost, ServiceError,
    VMHooks,
};
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
    vm_hooks: Rc<RefCell<Box<dyn VMHooks>>>,
    opcode_cost: Arc<Mutex<OpcodeCost>>,
}

impl WasmerExecutorData {
    pub fn new(vm_hooks: Box<dyn VMHooks>) -> Self {
        Self {
            vm_hooks: Rc::new(RefCell::new(vm_hooks)),
            opcode_cost: Arc::new(Mutex::new(OpcodeCost::default())),
        }
    }

    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError> {
        if let Some(vm_hooks) = Rc::get_mut(&mut self.vm_hooks) {
            vm_hooks.borrow_mut().set_vm_hooks_ptr(vm_hooks_ptr);
            Ok(())
        } else {
            Err(Box::new(ServiceError::new(
                "WasmerExecutor already set vmhooks, further configuration not allowed",
            )))
        }
    }

    fn set_opcode_cost(&mut self, opcode_cost: &OpcodeCost) -> Result<(), ExecutorError> {
        self.opcode_cost.lock().unwrap().clone_from(opcode_cost);
        Ok(())
    }

    pub(crate) fn get_vm_hooks(&self) -> Rc<RefCell<Box<dyn VMHooks>>> {
        self.vm_hooks.clone()
    }

    pub(crate) fn get_opcode_cost(&self) -> Arc<Mutex<OpcodeCost>> {
        self.opcode_cost.clone()
    }
}

pub struct WasmerExecutor {
    data: Rc<RefCell<WasmerExecutorData>>,
}

impl WasmerExecutor {
    pub fn new(vm_hooks: Box<dyn VMHooks>) -> Self {
        Self {
            data: Rc::new(RefCell::new(WasmerExecutorData::new(vm_hooks))),
        }
    }
}

impl ExecutorFull for WasmerExecutor {
    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError> {
        trace!("Setting vmhooks ...");
        self.data.borrow_mut().set_vm_hooks_ptr(vm_hooks_ptr)
    }

    fn set_opcode_cost(&mut self, opcode_cost: &OpcodeCost) -> Result<(), ExecutorError> {
        trace!("Setting opcode cost...");
        self.data.borrow_mut().set_opcode_cost(opcode_cost)
    }

    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn InstanceFull>, ExecutorError> {
        let instance =
            WasmerInstance::try_new_instance(self.data.clone(), wasm_bytes, compilation_options)?;
        Ok(Box::new(instance))
    }

    fn new_instance_from_cache(
        &self,
        cache_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn InstanceFull>, ExecutorError> {
        let instance = WasmerInstance::try_new_instance_from_cache(
            self.data.clone(),
            cache_bytes,
            compilation_options,
        )?;
        Ok(Box::new(instance))
    }
}
