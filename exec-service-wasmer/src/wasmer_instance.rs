use std::{cell::RefCell, ffi::c_void, mem::forget, rc::Rc};

use elrond_exec_service::{ExecutorError, ExecutorService, ServiceInstance};

use wasmer::{imports, wat2wasm, Extern, Instance, Module, Store, Value};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_engine_universal::Universal;

use crate::{
    wasmer_convert::convert_imports,
    wasmer_env::{ImportRuntimeContext, ImportRuntimeContextRef},
    BasicExecutorService, WasmerContext,
};

pub struct WasmerInstance {
    // pub(crate) service_ref: Rc<RefCell<Box<dyn ExecutorService>>>,
    pub(crate) context_rc: Rc<RefCell<WasmerContext>>,
    pub(crate) wasmer_instance: Instance,
    pub(crate) runtime_context_ref: ImportRuntimeContextRef, // TODO: move somewhere else, ideally some overarching context obj
}

impl WasmerInstance {
    pub fn new(
        context_rc: Rc<RefCell<WasmerContext>>,
        wasm_bytes: &[u8],
    ) -> Result<Self, ExecutorError> {
        // Use Singlepass compiler with the default settings
        let compiler = Singlepass::default();

        // Create the store
        let store = Store::new(&Universal::new(compiler).engine());

        println!("Compiling module...");
        // Let's compile the Wasm module.
        let module = Module::new(&store, wasm_bytes)?;

        // Create an empty import object.
        // let runtime_context = ImportRuntimeContext::default();
        println!("Converting imports...");
        let runtime_context_ref = ImportRuntimeContextRef::new(ImportRuntimeContext::default());
        let import_object = convert_imports(
            &store,
            runtime_context_ref.clone(),
            &context_rc.borrow().imports,
        );

        println!("Instantiating module...");
        // Let's instantiate the Wasm module.
        let wasmer_instance = Instance::new(&module, &import_object)?;

        Ok(WasmerInstance {
            context_rc,
            wasmer_instance,
            runtime_context_ref,
        })
    }
}

impl ServiceInstance for WasmerInstance {
    fn set_context_data_ptr(&mut self, context_ptr: *mut c_void) {
        println!("Setting context_ptr ... {:?}", context_ptr);
        self.runtime_context_ref.set_context_ptr(context_ptr);
        // self.context_ptr = context_ptr;
    }

    fn call(&self, func_name: &str) -> Result<(), String> {
        self.context_rc
            .borrow_mut()
            .push_execution_info(format!("Rust instance call! {}", func_name).as_str());

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
