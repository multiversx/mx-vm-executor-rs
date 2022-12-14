use crate::{
    wasmer_breakpoints::*, wasmer_imports::generate_import_object, wasmer_metering::*,
    wasmer_opcode_control::OpcodeControl, wasmer_vm_hooks::VMHooksWrapper, WasmerExecutorData,
};
use elrond_exec_service::{CompilationOptions, ExecutorError, Instance, ServiceError};
use std::{rc::Rc, sync::Arc};
use wasmer::Singlepass;
use wasmer::Universal;
use wasmer::{CompilerConfig, Extern, Module, Store};

pub struct WasmerInstance {
    pub(crate) executor_data: Rc<WasmerExecutorData>,
    pub(crate) wasmer_instance: wasmer::Instance,
    memory_name: String,
}

impl WasmerInstance {
    pub(crate) fn try_new_instance(
        executor_data: Rc<WasmerExecutorData>,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        // Use Singlepass compiler with the default settings
        let mut compiler = Singlepass::default();

        // Push middlewares
        push_middlewares(&mut compiler, compilation_options, executor_data.clone());

        // Create the store
        let store = Store::new(&Universal::new(compiler).engine());

        executor_data.print_execution_info("Compiling module ...");
        let module = Module::new(&store, wasm_bytes)?;

        // Create an empty import object.
        executor_data.print_execution_info("Converting imports ...");
        let vm_hooks_wrapper = VMHooksWrapper {
            vm_hooks: executor_data.vm_hooks.clone(),
        };
        let import_object = generate_import_object(&store, &vm_hooks_wrapper);

        executor_data.print_execution_info("Instantiating module ...");
        let wasmer_instance = wasmer::Instance::new(&module, &import_object)?;
        set_points_limit(&wasmer_instance, compilation_options.gas_limit);

        let memory_name = extract_wasmer_memory_name(&wasmer_instance)?;

        Ok(Box::new(WasmerInstance {
            executor_data,
            wasmer_instance,
            memory_name,
        }))
    }

    pub(crate) unsafe fn try_new_instance_from_cache(
        executor_data: Rc<WasmerExecutorData>,
        cache_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        // Use Singlepass compiler with the default settings
        let mut compiler = Singlepass::default();

        // Push middlewares
        push_middlewares(&mut compiler, compilation_options, executor_data.clone());

        // Create the store
        let store = Store::new(&Universal::new(compiler).engine());

        executor_data.print_execution_info("Deserializing module ...");
        let module = Module::deserialize(&store, cache_bytes)?;

        // Create an empty import object.
        executor_data.print_execution_info("Converting imports ...");
        let vm_hooks_wrapper = VMHooksWrapper {
            vm_hooks: executor_data.vm_hooks.clone(),
        };
        let import_object = generate_import_object(&store, &vm_hooks_wrapper);

        executor_data.print_execution_info("Instantiating module ...");
        let wasmer_instance = wasmer::Instance::new(&module, &import_object)?;
        set_points_limit(&wasmer_instance, compilation_options.gas_limit);

        let memory_name = extract_wasmer_memory_name(&wasmer_instance)?;

        Ok(Box::new(WasmerInstance {
            executor_data,
            wasmer_instance,
            memory_name,
        }))
    }

    fn get_memory_ref(&self) -> &wasmer::Memory {
        self.wasmer_instance
            .exports
            .get_memory(&self.memory_name)
            .unwrap_or_else(|_| {
                panic!("memory name not found, should not happen since it was already checked")
            })
    }
}

fn extract_wasmer_memory_name(wasmer_instance: &wasmer::Instance) -> Result<String, ExecutorError> {
    let memories = wasmer_instance
        .exports
        .iter()
        .memories()
        .collect::<Vec<_>>();
    if memories.len() > 1 {
        return Err(Box::new(ServiceError::new(
            "more than one memory declared in smart contract",
        )));
    }

    if let Some(entry) = memories.get(0) {
        Ok(entry.0.clone())
    } else {
        Err(Box::new(ServiceError::new(
            "no memory declared in smart contract",
        )))
    }
}

fn push_middlewares(
    compiler: &mut Singlepass,
    compilation_options: &CompilationOptions,
    executor_data: Rc<WasmerExecutorData>,
) {
    // Create breakpoint middelware
    let breakpoints_middleware = Arc::new(Breakpoints::new());

    // Create opcode_control middleware
    let opcode_control_middleware = Arc::new(OpcodeControl::new(
        compilation_options.max_memory_grow,
        compilation_options.max_memory_grow_delta,
        breakpoints_middleware.clone(),
    ));

    // Create metering middleware
    let metering_middleware = Arc::new(Metering::new(
        compilation_options.gas_limit,
        executor_data.opcode_cost.clone(),
        breakpoints_middleware.clone(),
    ));

    executor_data.print_execution_info("Adding metering middleware ...");
    compiler.push_middleware(metering_middleware);
    executor_data.print_execution_info("Adding opcode_control middleware ...");
    compiler.push_middleware(opcode_control_middleware);
    executor_data.print_execution_info("Adding breakpoints middleware ...");
    compiler.push_middleware(breakpoints_middleware);
}

impl Instance for WasmerInstance {
    fn call(&self, func_name: &str) -> Result<(), String> {
        self.executor_data
            .print_execution_info(format!("Rust instance call: {}", func_name).as_str());

        let func = self
            .wasmer_instance
            .exports
            .get_function(func_name)
            .map_err(|_| "function not found".to_string())?;

        match func.call(&[]) {
            Ok(_) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }

    fn check_signatures(&self) -> bool {
        for (_, export) in self.wasmer_instance.exports.iter() {
            if let Extern::Function(endpoint) = export {
                if endpoint.param_arity() > 0 {
                    return false;
                }
                if endpoint.result_arity() > 0 {
                    return false;
                }
            }
        }

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

    fn set_points_limit(&self, limit: u64) {
        set_points_limit(&self.wasmer_instance, limit)
    }

    fn set_points_used(&self, points: u64) {
        set_points_used(&self.wasmer_instance, points)
    }

    fn get_points_used(&self) -> u64 {
        get_points_used(&self.wasmer_instance)
    }

    fn memory_length(&self) -> u64 {
        self.get_memory_ref().data_size()
    }

    fn memory_ptr(&self) -> *mut u8 {
        self.get_memory_ref().data_ptr()
    }

    fn memory_grow(&self, by_num_pages: u32) -> Result<u32, ExecutorError> {
        let pages = self.get_memory_ref().grow(wasmer::Pages(by_num_pages))?;
        Ok(pages.0)
    }

    fn set_breakpoint_value(&self, value: u64) {
        set_breakpoint_value(&self.wasmer_instance, value)
    }

    fn get_breakpoint_value(&self) -> u64 {
        get_breakpoint_value(&self.wasmer_instance)
    }

    fn cache(
        &self,
        cache_bytes_ptr: *mut *const u8,
        cache_bytes_len: *mut u32,
    ) -> Result<(), String> {
        let module = self.wasmer_instance.module();
        match module.serialize() {
            Ok(bytes) => {
                unsafe {
                    *cache_bytes_ptr = bytes.as_ptr();
                }
                unsafe {
                    *cache_bytes_len = bytes.len() as u32;
                }
                std::mem::forget(bytes);
                Ok(())
            }
            Err(err) => Err(err.to_string()),
        }
    }

    fn reset(&self) -> Result<(), String> {
        self.wasmer_instance.reset()
    }
}
