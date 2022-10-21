use crate::{
    wasmer_imports::generate_import_object, wasmer_vm_hooks::VMHooksWrapper, WasmerContext,
    WasmerInstance,
};
use elrond_exec_service::{
    CompilationOptions, Executor, ExecutorError, Instance, ServiceError, VMHooks,
};
use std::ffi::c_void;
use std::{cell::RefCell, rc::Rc};
use wasmer::{Module, Store};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_engine_universal::Universal;

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
        _compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        // Use Singlepass compiler with the default settings
        let compiler = Singlepass::default();

        // Create the store
        let store = Store::new(&Universal::new(compiler).engine());

        println!("Compiling module ...");
        let module = Module::new(&store, wasm_bytes)?;

        // Create an empty import object.
        println!("Converting imports ...");
        let vm_hooks_wrapper = VMHooksWrapper {
            vm_hooks: self.data.vm_hooks.clone(),
            // vm_hooks: Rc::new(Box::new(VMHooksDefault)),
        };
        let import_object = generate_import_object(&store, &vm_hooks_wrapper);

        println!("Instantiating module ...");
        let wasmer_instance = wasmer::Instance::new(&module, &import_object)?;

        Ok(Box::new(WasmerInstance {
            executor_data: self.data.clone(),
            context_rc: Rc::new(RefCell::new(WasmerContext::default())),
            wasmer_instance,
        }))
    }
}
