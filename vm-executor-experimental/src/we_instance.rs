use crate::middlewares::{
    Breakpoints, Metering, OpcodeControl, OpcodeTracer, ProtectedGlobals, get_breakpoint_value,
    get_points_used, set_points_limit,
};
use crate::{ExperimentalError, ExperimentalVMHooksBuilder};
use crate::{we_imports::generate_import_object, we_vm_hooks::VMHooksWrapper};
use log::trace;
use multiversx_chain_vm_executor::{
    BreakpointValue, CompilationOptions, ExecutorError, Instance, InstanceCallResult, OpcodeCost,
    ServiceError, VMHooksEarlyExit,
};
use rc_new_cyclic_fallible::rc_new_cyclic_fallible;
use std::{rc::Rc, rc::Weak, sync::Arc};
use wasmer::sys::{CompilerConfig, Singlepass};
use wasmer::{Extern, Module, Pages, Store};

const MAX_MEMORY_PAGES_ALLOWED: Pages = Pages(20);

pub struct ExperimentalInstance {
    wasmer_store: wasmer::Store,
    inner: Rc<ExperimentalInstanceInner>,
}

pub struct ExperimentalInstanceInner {
    pub wasmer_instance: wasmer::Instance,
    pub memory_name: String,
}

impl ExperimentalInstanceInner {
    pub fn get_memory_ref(&self) -> Result<&wasmer::Memory, String> {
        let result = self.wasmer_instance.exports.get_memory(&self.memory_name);
        match result {
            Ok(memory) => Ok(memory),
            Err(err) => Err(err.to_string()),
        }
    }
}

fn prepare_wasmer_instance_inner(
    vm_hooks_builder: Box<dyn ExperimentalVMHooksBuilder>,
    module: &Module,
    store: &mut Store,
    weak: &Weak<ExperimentalInstanceInner>,
) -> Result<ExperimentalInstanceInner, ExecutorError> {
    // Create an empty import object.
    trace!("Generating imports ...");
    let vm_hooks_wrapper = VMHooksWrapper {
        vm_hooks_builder,
        wasmer_inner: weak.clone(),
    };
    let import_object = generate_import_object(store, vm_hooks_wrapper);

    trace!("Instantiating WasmerInstance ...");
    let wasmer_instance = wasmer::Instance::new(store, module, &import_object)?;

    // Check that there is exactly one memory in the smart contract, no more, no less
    let memories = get_memories(&wasmer_instance);
    validate_memories(&memories)?;

    // At this point we know that there is exactly one memory
    let memory = memories[0].1;
    // Checks that the memory size is not greater than the maximum allowed
    validate_memory(memory, store)?;

    trace!("WasmerMemory size: {:#?}", memory.view(&store).size());
    let memory_name = memories[0].0.clone();

    Ok(ExperimentalInstanceInner {
        wasmer_instance,
        memory_name,
    })
}

impl ExperimentalInstance {
    pub fn try_new_instance(
        vm_hooks_builder: Box<dyn ExperimentalVMHooksBuilder>,
        opcode_cost: Arc<OpcodeCost>,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Self, ExecutorError> {
        // Use Singlepass compiler with the default settings
        let mut compiler = Singlepass::default();

        // Push middlewares
        push_middlewares(&mut compiler, compilation_options, opcode_cost);

        // Create the store
        let mut store: Store = Store::new(compiler);

        trace!("Compiling module ...");
        let module = Module::new(&store, wasm_bytes)?;

        let inner = rc_new_cyclic_fallible(|weak| {
            prepare_wasmer_instance_inner(vm_hooks_builder, &module, &mut store, weak)
        })?;

        Ok(ExperimentalInstance {
            wasmer_store: store,
            inner,
        })
    }

    fn get_breakpoint_value(&mut self) -> Result<BreakpointValue, ExecutorError> {
        let value = get_breakpoint_value(&self.inner.wasmer_instance, &mut self.wasmer_store)?;
        value
            .try_into()
            .map_err(|_| ExperimentalError::UnknownBreakpointValue(value).into())
    }
}

fn get_memories(wasmer_instance: &wasmer::Instance) -> Vec<(&String, &wasmer::Memory)> {
    let memories = wasmer_instance
        .exports
        .iter()
        .memories()
        .collect::<Vec<_>>();
    memories
}

fn validate_memories(memories: &[(&String, &wasmer::Memory)]) -> Result<(), ExecutorError> {
    if memories.is_empty() {
        return Err(Box::new(ServiceError::new(
            "no memory declared in smart contract",
        )));
    }
    if memories.len() > 1 {
        return Err(Box::new(ServiceError::new(
            "more than one memory declared in smart contract",
        )));
    }

    Ok(())
}

fn validate_memory(memory: &wasmer::Memory, store: &wasmer::Store) -> Result<(), ExecutorError> {
    let memory_type = memory.ty(store);
    let max_memory_pages = memory_type.maximum.unwrap_or(memory_type.minimum);

    if max_memory_pages > MAX_MEMORY_PAGES_ALLOWED {
        trace!(
            "Memory size exceeds maximum allowed: {:#?} > {:#?}",
            max_memory_pages, MAX_MEMORY_PAGES_ALLOWED
        );
        return Err(Box::new(ServiceError::new(
            "memory size exceeds maximum allowed",
        )));
    }

    Ok(())
}

fn push_middlewares(
    compiler: &mut Singlepass,
    compilation_options: &CompilationOptions,
    opcode_cost: Arc<OpcodeCost>,
) {
    // Create breakpoints middleware
    let breakpoints_middleware = Arc::new(Breakpoints::new());

    // Create opcode_control middleware
    let opcode_control_middleware = Arc::new(OpcodeControl::new(
        100, // TODO: should be compilation_options.max_memory_grow_count,
        compilation_options.max_memory_grow,
        compilation_options.max_memory_grow_delta,
        breakpoints_middleware.clone(),
    ));

    // Create metering middleware
    let metering_middleware = Arc::new(Metering::new(
        compilation_options.unmetered_locals,
        opcode_cost,
        breakpoints_middleware.clone(),
    ));

    // Create protected_globals middleware
    let protected_globals_middleware = Arc::new(ProtectedGlobals::new(vec![
        breakpoints_middleware.clone(),
        metering_middleware.clone(),
    ]));

    trace!("Adding protected_globals middleware ...");
    compiler.push_middleware(protected_globals_middleware);
    trace!("Adding metering middleware ...");
    compiler.push_middleware(metering_middleware);
    trace!("Adding opcode_control middleware ...");
    compiler.push_middleware(opcode_control_middleware);
    trace!("Adding breakpoints middleware ...");
    compiler.push_middleware(breakpoints_middleware);

    if compilation_options.opcode_trace {
        // Create opcode_tracer middleware
        let opcode_tracer_middleware = Arc::new(OpcodeTracer::new());
        trace!("Adding opcode_tracer middleware ...");
        compiler.push_middleware(opcode_tracer_middleware);
    }
}

impl Instance for ExperimentalInstance {
    fn call(&mut self, func_name: &str, points_limit: u64) -> InstanceCallResult {
        trace!("Rust instance call: {func_name}");

        let Ok(func) = self.inner.wasmer_instance.exports.get_function(func_name) else {
            return InstanceCallResult::FunctionNotFound;
        };

        if let Err(err) = set_points_limit(
            &self.inner.wasmer_instance,
            &mut self.wasmer_store,
            points_limit,
        ) {
            return InstanceCallResult::RuntimeError(err);
        }

        match func.call(&mut self.wasmer_store, &[]) {
            Ok(_) => {
                trace!("Call succeeded: {func_name}");
                InstanceCallResult::Ok
            }
            Err(runtime_error) => {
                trace!("Call failed: {func_name} - {runtime_error}");

                match runtime_error.downcast::<VMHooksEarlyExit>() {
                    Ok(vm_hooks_error) => InstanceCallResult::VMHooksEarlyExit(vm_hooks_error),
                    Err(other_error) => {
                        let breakpoint = self
                            .get_breakpoint_value()
                            .expect("error retrieving instance breakpoint value");
                        if breakpoint != BreakpointValue::None {
                            InstanceCallResult::Breakpoint(breakpoint)
                        } else {
                            InstanceCallResult::RuntimeError(
                                ExperimentalError::InstanceCall(other_error).into(),
                            )
                        }
                    }
                }
            }
        }
    }

    fn check_signatures(&self) -> bool {
        for (_, export) in self.inner.wasmer_instance.exports.iter() {
            if let Extern::Function(endpoint) = export {
                if endpoint.param_arity(&self.wasmer_store) > 0 {
                    return false;
                }
                if endpoint.result_arity(&self.wasmer_store) > 0 {
                    return false;
                }
            }
        }

        true
    }

    fn has_function(&self, func_name: &str) -> bool {
        self.inner
            .wasmer_instance
            .exports
            .get_function(func_name)
            .is_ok()
    }

    fn get_exported_function_names(&self) -> Vec<String> {
        self.inner
            .wasmer_instance
            .exports
            .iter()
            .filter_map(|(name, export)| match export {
                Extern::Function(_) => Some(name),
                _ => None,
            })
            .cloned()
            .collect()
    }

    fn get_points_used(&mut self) -> Result<u64, ExecutorError> {
        get_points_used(&self.inner.wasmer_instance, &mut self.wasmer_store)
    }

    fn reset(&self) -> Result<(), ExecutorError> {
        panic!("ExperimentalInstance reset not supported")
    }

    fn cache(&self) -> Result<Vec<u8>, ExecutorError> {
        let module = self.inner.wasmer_instance.module();
        Ok(module.serialize()?.to_vec())
    }
}
