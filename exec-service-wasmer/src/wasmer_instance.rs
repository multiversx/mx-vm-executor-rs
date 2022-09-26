use std::{cell::RefCell, rc::Rc};

use elrond_exec_service::{ExecutorError, ExecutorService, ServiceInstance};

use wasmer::{imports, wat2wasm, Instance, Module, Store, Value};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_engine_universal::Universal;

use crate::{BasicExecutorService, WasmerContext};

pub struct WasmerInstance {
    // pub(crate) service_ref: Rc<RefCell<Box<dyn ExecutorService>>>,
    pub(crate) context_rc: Rc<RefCell<WasmerContext>>,
    pub(crate) instance: Instance,
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
        let import_object = imports! {};

        println!("Instantiating module...");
        // Let's instantiate the Wasm module.
        let instance = Instance::new(&module, &import_object)?;

        Ok(WasmerInstance {
            context_rc,
            instance,
        })
    }
}

impl ServiceInstance for WasmerInstance {
    fn call(&self, func_name: &str) -> Result<(), String> {
        self.context_rc
            .borrow_mut()
            .push_execution_info(format!("Rust instance call! {}", func_name).as_str());
        Ok(())
    }
}
