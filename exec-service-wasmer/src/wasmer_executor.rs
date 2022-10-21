use crate::WasmerInstance;
use elrond_exec_service::{
    CompilationOptions, Executor, ExecutorError, Instance, ServiceError, VMHooks,
};
use std::ffi::c_void;
use std::rc::Rc;

pub struct WasmerExecutor {
    pub data: Rc<WasmerExecutorData>,
}

pub struct WasmerExecutorData {
    pub vm_hooks: Rc<Box<dyn VMHooks>>,
}

impl Executor for WasmerExecutor {
    fn set_vm_hooks_ptr(&mut self, vm_hooks_ptr: *mut c_void) -> Result<(), ExecutorError> {
        println!("Setting context pointer ...");
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

    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        WasmerInstance::new(self.data.clone(), wasm_bytes, compilation_options)
    }
}
