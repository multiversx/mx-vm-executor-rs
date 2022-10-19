use crate::{
    wasmer_imports::generate_import_object, wasmer_vm_hooks::VMHooksWrapper, BasicExecutorService,
    WasmerContext, WasmerInstance,
};
use elrond_exec_service::{
    CompilationOptions, Executor, ExecutorError, ExecutorService, ServiceInstance, VMHooksDefault,
};
use std::{cell::RefCell, rc::Rc};
use wasmer::{imports, wat2wasm, Extern, Instance, Module, Store, Value};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_engine_universal::Universal;

pub struct WasmerExecutor {
    pub data: Rc<WasmerExecutorData>,
}

#[derive(Default)]
pub struct WasmerExecutorData {}

impl Executor for WasmerExecutor {
    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        _compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn ServiceInstance>, ExecutorError> {
        // Use Singlepass compiler with the default settings
        let compiler = Singlepass::default();

        // Create the store
        let store = Store::new(&Universal::new(compiler).engine());

        println!("Compiling module...");
        let module = Module::new(&store, wasm_bytes)?;

        // Create an empty import object.
        println!("Converting imports...");
        let vm_hooks_wrapper = VMHooksWrapper::new(VMHooksDefault);
        let import_object = generate_import_object(&store, &vm_hooks_wrapper);

        println!("Instantiating module...");
        let wasmer_instance = Instance::new(&module, &import_object)?;

        Ok(Box::new(WasmerInstance {
            executor_data: self.data.clone(),
            context_rc: Rc::new(RefCell::new(WasmerContext::default())),
            wasmer_instance,
            vm_hooks_wrapper,
        }))
    }
}
