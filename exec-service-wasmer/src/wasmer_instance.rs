use crate::{
    wasmer_imports::generate_import_object, wasmer_metering::*, wasmer_vm_hooks::VMHooksWrapper,
    WasmerExecutorData,
};
use elrond_exec_service::{CompilationOptions, ExecutorError, Instance};
use std::{rc::Rc, sync::Arc};
use wasmer::{wasmparser::Operator, CompilerConfig, Extern, Module, Store};
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
        // Let's define our cost function.
        //
        // This function will be called for each `Operator` encountered during
        // the Wasm module execution. It should return the cost of the operator
        // that it received as it first argument.
        let cost_function = |operator: &Operator| -> u64 {
            match operator {
                Operator::LocalGet { .. } | Operator::I32Const { .. } => 1,
                Operator::I32Add { .. } => 2,
                _ => 0,
            }
        };

        // Now let's create our metering middleware.
        //
        // `Metering` needs to be configured with a limit and a cost function.
        //
        // For each `Operator`, the metering middleware will call the cost
        // function and subtract the cost from the remaining points.
        // for testing purposes, we set the limit to 10_000
        let initial_limit = 10_000;
        //let initial_limit = compilation_options.gas_limit;
        let metering = Arc::new(Metering::new(initial_limit, cost_function));

        // Use Singlepass compiler with the default settings
        let mut compiler = Singlepass::default();
        compiler.push_middleware(metering);
        println!("Added metering middleware");

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

    // metering
    fn get_points_used(&self) -> u64 {
        0
    }

    fn set_points_used(&self, _new_gas: u64) {}

    fn set_points_limit(&self, new_limit: u64) {
        set_remaining_points(&self.wasmer_instance, new_limit)
    }
}
