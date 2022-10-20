use std::{cell::RefCell, ffi::c_void, mem::forget, rc::Rc};

use elrond_exec_service::{
    Executor, ExecutorError, ExecutorService, ServiceInstance, VMHooksDefault,
};

use wasmer::{imports, wat2wasm, Extern, Instance, Module, Store, Value};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_engine_universal::Universal;

use crate::{
    wasmer_imports::generate_import_object, wasmer_vm_hooks::VMHooksWrapper, BasicExecutorService,
    WasmerContext, WasmerExecutorData,
};

pub struct WasmerInstance {
    pub executor_data: Rc<WasmerExecutorData>,
    pub context_rc: Rc<RefCell<WasmerContext>>, // TODO: remove
    pub wasmer_instance: Instance,
}

impl WasmerInstance {
    // pub fn new(
    //     context_rc: Rc<RefCell<WasmerContext>>,
    //     wasm_bytes: &[u8],
    // ) -> Result<Self, ExecutorError> {
    // }
}

impl ServiceInstance for WasmerInstance {
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
