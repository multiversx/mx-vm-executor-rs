use crate::ExperimentalError;
use crate::middlewares::{
    Breakpoints, Metering, OpcodeControl, OpcodeTracer, ProtectedGlobals, get_breakpoint_value,
    get_points_used, set_points_limit,
};
use crate::{we_imports::generate_import_object, we_vm_hooks::VMHooksWrapper};
use log::trace;
use multiversx_chain_vm_executor::{
    BreakpointValue, CompilationOptions, ExecutorError, ExecutorRuntime, Instance,
    InstanceCallResult, ServiceError, VMHooksEarlyExit,
};
use rc_new_cyclic_fallible::rc_new_cyclic_fallible;
use std::{rc::Rc, rc::Weak, sync::Arc};
use wasmer::sys::{CompilerConfig, Singlepass};
use wasmer::{Extern, ExternType, Module, Pages, Store};

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
    runtime: Arc<dyn ExecutorRuntime>,
    module: &Module,
    store: &mut Store,
    weak: &Weak<ExperimentalInstanceInner>,
) -> Result<ExperimentalInstanceInner, ExecutorError> {
    // Create an empty import object.
    trace!("Generating imports ...");
    let vm_hooks_wrapper = VMHooksWrapper {
        runtime,
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

/// Builds the store the module will be compiled into, or deserialized against.
///
/// The middlewares are pushed here rather than at instantiation because they are
/// *compile-time* transforms: they rewrite the module and add the metering and breakpoint
/// globals. A deserialized module already carries that instrumentation, so pushing them again
/// changes nothing about it — what matters on that path is only that the engine agrees with
/// the artifact, which is why both paths build the store the same way.
///
/// One store per instance, and so one engine per compiled module. The metering middleware
/// keeps its global indexes in a `Mutex<Option<..>>` filled in during compilation, so
/// compiling a second contract through the same engine would overwrite the first one's. See
/// `PLAN-rust-vm-caching.md`, 1.1.
fn new_store(compilation_options: &CompilationOptions) -> Store {
    // Use Singlepass compiler with the default settings
    let mut compiler = Singlepass::default();

    // Push middlewares
    push_middlewares(&mut compiler, compilation_options);

    Store::new(compiler)
}

impl ExperimentalInstance {
    pub fn try_new_instance(
        runtime: Arc<dyn ExecutorRuntime>,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Self, ExecutorError> {
        let store = new_store(compilation_options);

        trace!("Compiling module ...");
        let module = Module::new(&store, wasm_bytes)?;

        Self::try_from_module(runtime, store, &module)
    }

    /// Instantiates a module from a previously serialized artifact, skipping compilation.
    ///
    /// The artifact comes from [`Instance::cache`], which is `Module::serialize`.
    ///
    /// # Trust
    ///
    /// `Module::deserialize` is **not** a validating load: in Wasmer 7.5 it delegates to
    /// `deserialize_unchecked`, so it maps whatever it is given as executable code. The name
    /// suggests otherwise, which is why it is said here. Artifacts must come from a store the
    /// host trusts as much as it trusts itself — for the Go VM that is its own compiled-code
    /// cache, written by `cache()` on this same machine.
    ///
    /// A mismatched artifact is rejected rather than misread, but only as far as the header
    /// goes: it carries a Wasmer ABI version (11 in 7.1, 24 in 7.4) that `deserialize`
    /// refuses on mismatch. So an upgrade invalidates a persisted cache loudly, as an error
    /// here, rather than silently — but a cache that outlives an upgrade still has to be
    /// keyed by Wasmer version or purged, because every entry in it will fail this way.
    pub fn try_new_instance_from_cache(
        runtime: Arc<dyn ExecutorRuntime>,
        cache_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Self, ExecutorError> {
        let store = new_store(compilation_options);

        trace!("Deserializing module ...");
        // SAFETY: see the trust note above — the caller vouches for the artifact's origin.
        let module = unsafe { Module::deserialize(&store, cache_bytes)? };

        Self::try_from_module(runtime, store, &module)
    }

    /// The half the two entry points share: everything after there is a `Module`.
    fn try_from_module(
        runtime: Arc<dyn ExecutorRuntime>,
        mut store: Store,
        module: &Module,
    ) -> Result<Self, ExecutorError> {
        let inner = rc_new_cyclic_fallible(|weak| {
            prepare_wasmer_instance_inner(runtime, module, &mut store, weak)
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
    wasmer_instance
        .exports
        .iter()
        .memories()
        .collect::<Vec<_>>()
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

fn push_middlewares(compiler: &mut Singlepass, compilation_options: &CompilationOptions) {
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
        compilation_options.opcode_config.clone(),
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
                        let breakpoint = match self.get_breakpoint_value() {
                            Ok(breakpoint) => breakpoint,
                            Err(err) => return InstanceCallResult::RuntimeError(err),
                        };
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

    fn get_imported_function_names(&self) -> Vec<String> {
        self.inner
            .wasmer_instance
            .module()
            .imports()
            .filter(|import| matches!(import.ty(), ExternType::Function(_)))
            .map(|import| import.name().to_string())
            .collect()
    }

    fn get_points_used(&mut self) -> Result<u64, ExecutorError> {
        get_points_used(&self.inner.wasmer_instance, &mut self.wasmer_store)
    }

    fn reset(&self) -> Result<(), ExecutorError> {
        Err(ExperimentalError::ResetUnsupported.into())
    }

    fn cache(&self) -> Result<Vec<u8>, ExecutorError> {
        let module = self.inner.wasmer_instance.module();
        Ok(module.serialize()?.to_vec())
    }
}
