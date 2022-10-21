use crate::{
    wasmer_imports::generate_import_object, wasmer_vm_hooks::VMHooksWrapper, WasmerExecutorData,
};
use elrond_exec_service::{CompilationOptions, ExecutorError, Instance};
use std::rc::Rc;
use wasmer::{Extern, Module, Store};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_engine_universal::Universal;

pub struct WasmerInstance {
    pub executor_data: Rc<WasmerExecutorData>,
    pub wasmer_instance: wasmer::Instance,
}

impl WasmerInstance {
    pub(crate) fn new(
        executor_data: Rc<WasmerExecutorData>,
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
            vm_hooks: executor_data.vm_hooks.clone(),
            // vm_hooks: Rc::new(Box::new(VMHooksDefault)),
        };
        let import_object = generate_import_object(&store, &vm_hooks_wrapper);

        println!("Instantiating module ...");
        let wasmer_instance = wasmer::Instance::new(&module, &import_object)?;

        Ok(Box::new(WasmerInstance {
            executor_data,
            wasmer_instance,
        }))
    }
}

impl Instance for WasmerInstance {
    fn call(&self, func_name: &str) -> Result<(), String> {
        println!("Rust instance call: {}", func_name);

        let func = self
            .wasmer_instance
            .exports
            .get_function(func_name)
            .map_err(|_| "function not found".to_string())?;

        let _ = func.call(&[]);

        Ok(())
    }

    fn check_signatures(&self) -> bool {
        true
    }

    fn has_function(&self, func_name: &str) -> bool {
        self.wasmer_instance.exports.get_function(func_name).is_ok()
    }

    fn get_exported_function_names(&self) -> Vec<String> {
        self.wasmer_instance
            .exports
            .iter()
            .filter_map(|(name, export)| match export {
                Extern::Function(_) => Some(name),
                _ => None,
            })
            .cloned()
            .collect()
    }
}
