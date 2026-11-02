use crate::ExperimentalError;
use crate::middlewares::{
    Breakpoints, Metering, OpcodeControl, OpcodeTracer, ProtectedGlobals, get_breakpoint_value,
    get_points_used, set_points_limit, set_points_used,
};
use crate::{we_imports::generate_import_object, we_vm_hooks::VMHooksWrapper};
use log::trace;
use multiversx_chain_vm_executor::{
    BreakpointValue, CompilationOptions, ExecutorError, ExecutorRuntime, Instance,
    InstanceCallResult, InstanceStopHandle, ServiceError, VMHooksEarlyExit,
};
use rc_new_cyclic_fallible::rc_new_cyclic_fallible;
use std::{
    rc::Rc,
    rc::Weak,
    sync::{Arc, Mutex},
};
use wasmer::sys::{CompilerConfig, Singlepass};
use wasmer::{Extern, ExternType, Module, Pages, Store};
use wasmer_types::TrapCode;

const MAX_MEMORY_PAGES_ALLOWED: Pages = Pages(20);

#[cfg(unix)]
use wasmer::Interrupter;

#[cfg(unix)]
fn store_interrupter(store: &Store) -> Interrupter {
    store.interrupter()
}

/// Stands in where Wasmer has no interrupter to offer: upstream it is unix-only, behind the
/// `experimental-host-interrupt` feature this crate enables unconditionally. Requesting a
/// stop is then a no-op and the call runs to completion, which the contract of
/// `request_stop` already allows, it being best effort.
#[cfg(not(unix))]
#[derive(Debug)]
struct Interrupter;

#[cfg(not(unix))]
impl Interrupter {
    fn interrupt(&self) {}
}

#[cfg(not(unix))]
fn store_interrupter(_store: &Store) -> Interrupter {
    Interrupter
}

/// One instantiated contract, and everything needed to put it back the way it started.
///
/// # What it owns
///
/// A store, an instance inside that store, and the two things a rebuild needs: the compiled
/// module and the runtime the imports are built from. The store is *per instance*, not per
/// executor — see [`new_store`] for why the metering middleware forces that — so this struct
/// is the whole of one contract's world: its memory, its globals, its tables, and the engine
/// its code was compiled by.
///
/// # Not `Sync`, not `Send`, and not reentrant
///
/// `inner` is an `Rc`, so an instance belongs to one thread; the Go VM honours that because
/// cgo pins a goroutine to its OS thread for the duration of a call.
///
/// Reentrancy is the caller's obligation, not something checked here: every entry point that
/// can touch the store takes `&mut self`, so calling one while a call is in flight would
/// alias. Nesting into a *different* instance is fine and is what the Go VM does for
/// `executeOnDestContext`; re-entering the same one is refused by the host, in
/// `Wasmer7Instance.inCall`.
///
/// # The back-reference, and why `inner` is separate
///
/// The import object a hook is dispatched through has to be able to find the instance's
/// memory, but it is built *before* the instance exists. [`ExperimentalInstanceInner`] is
/// the knot that ties: it is created through `rc_new_cyclic_fallible`, so the imports can
/// capture a `Weak` to it that becomes valid the moment instantiation succeeds. That is the
/// only reason the wasmer instance is not simply a field here.
pub struct ExperimentalInstance {
    /// Owns the instance's memory, globals and tables, and the engine its module was
    /// compiled by. Replaced wholesale by [`Instance::reset`].
    wasmer_store: wasmer::Store,

    /// The interrupter for the current store, shared with every outstanding stop handle.
    ///
    /// Behind an `Arc<Mutex<_>>` rather than handed out by value because an interrupter names
    /// one store, and [`Instance::reset`] builds a new one. A handle taken before a reset
    /// would otherwise go quietly dead; this way the reset refreshes what every handle reads.
    interrupter: Arc<Mutex<Interrupter>>,

    /// The wasmer instance, behind the `Rc` the import object holds a `Weak` to.
    inner: Rc<ExperimentalInstanceInner>,

    /// Kept so the instance can be rebuilt in place by [`Instance::reset`].
    ///
    /// Cheap to hold — `Module` is a handle over refcounted compiled code, not a copy of it —
    /// and it is the only way to re-instantiate without going back to the wasm bytes. It is
    /// also what [`Instance::cache`] serializes.
    module: Module,

    /// Kept for the same reason: re-instantiating means building a fresh import object, and
    /// that needs the runtime the hooks are reached through.
    ///
    /// An `Arc<dyn ExecutorRuntime>` rather than a concrete type, because the runtime is the
    /// host's — the Go VM's hook bridge, or the Rust VM's — and the backend must not know
    /// which. It is cloned into each import object, so the imports and this struct keep the
    /// runtime alive between them for as long as either exists.
    runtime: Arc<dyn ExecutorRuntime>,
}

/// The part of an instance that a hook's import object can hold a `Weak` to.
///
/// Split out of [`ExperimentalInstance`] for one reason, given in that type's documentation:
/// the import object has to be built before the instance it belongs to exists, so the back
/// reference has to go through something tied by `rc_new_cyclic_fallible`.
pub struct ExperimentalInstanceInner {
    pub wasmer_instance: wasmer::Instance,

    /// The export name of the contract's single memory, resolved once at instantiation.
    ///
    /// Held rather than looked up per access because every hook that touches memory would
    /// otherwise re-scan the export list, and because the "exactly one memory" rule is
    /// checked at instantiation — recording the name is what carries that check forward.
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

        Self::try_from_module(runtime, store, module)
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

        Self::try_from_module(runtime, store, module)
    }

    /// The half the two entry points share: everything after there is a `Module`.
    fn try_from_module(
        runtime: Arc<dyn ExecutorRuntime>,
        mut store: Store,
        module: Module,
    ) -> Result<Self, ExecutorError> {
        let inner = rc_new_cyclic_fallible(|weak| {
            prepare_wasmer_instance_inner(runtime.clone(), &module, &mut store, weak)
        })?;

        Ok(ExperimentalInstance {
            interrupter: Arc::new(Mutex::new(store_interrupter(&store))),
            wasmer_store: store,
            inner,
            module,
            runtime,
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
                        // A host-requested stop arrives as a trap, not as a global: the
                        // interrupter delivers it by signal, so there is nothing to poll.
                        if other_error.clone().to_trap() == Some(TrapCode::HostInterrupt) {
                            return InstanceCallResult::Interrupted;
                        }

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

    fn set_points_used(&mut self, points: u64) -> Result<(), ExecutorError> {
        set_points_used(&self.inner.wasmer_instance, &mut self.wasmer_store, points)
    }

    /// Re-instantiates the module into a fresh store, in place.
    ///
    /// Wasmer 7 has no `Instance::reset` — the only `reset` it offers is `Memory::reset`,
    /// which empties the memory and restores neither its contents nor the globals — so there
    /// is nothing to rewind an instance with. What there is, is the module: instantiating it
    /// again produces exactly the state a fresh instance would have, because that is the same
    /// operation. So this throws the instance away and builds another one behind the same
    /// handle.
    ///
    /// **The store is rebuilt, not reused.** It owns the instance's memory and globals, so
    /// keeping it would keep the very state being discarded. The *engine* is reused — cloned
    /// out of the old store, which is a refcount bump — and that is what makes this cheap:
    /// the module stays compiled, and only instantiation is repeated.
    ///
    /// Compared with the Wasmer 2.2 fork's `Instance::reset`, which rewinds globals and
    /// memories in place, this is slower — it re-runs the whole instantiation, including
    /// rebuilding the import object. It is also stricter: tables and passive-element
    /// bookkeeping come back too, which the fork's version does not restore. See
    /// `PLAN-reset-via-middlewares.md` for the faster in-place design and why it was not
    /// built yet.
    ///
    /// On failure the instance is left untouched rather than half-rebuilt, since the new
    /// store and inner are only swapped in once both exist.
    fn reset(&mut self) -> Result<(), ExecutorError> {
        // Same engine, so the already-compiled module is still valid in the new store.
        let mut store = Store::new(self.wasmer_store.engine().clone());

        let runtime = self.runtime.clone();
        let module = self.module.clone();
        let inner = rc_new_cyclic_fallible(|weak| {
            prepare_wasmer_instance_inner(runtime, &module, &mut store, weak)
        })?;

        // Dropping the old store releases the old instance's memory and globals, and with
        // them the import object holding the previous `Weak` back-reference.
        // Before the old store is dropped, so that a handle taken earlier starts pointing at
        // the new store rather than at one that no longer exists.
        if let Ok(mut interrupter) = self.interrupter.lock() {
            *interrupter = store_interrupter(&store);
        }

        self.wasmer_store = store;
        self.inner = inner;

        Ok(())
    }

    fn cache(&self) -> Result<Vec<u8>, ExecutorError> {
        let module = self.inner.wasmer_instance.module();
        Ok(module.serialize()?.to_vec())
    }

    fn stop_handle(&self) -> Box<dyn InstanceStopHandle> {
        Box::new(ExperimentalStopHandle {
            interrupter: self.interrupter.clone(),
        })
    }
}

/// Stops a Wasmer 7 call through the store's interrupter.
///
/// Where the 2.2 backend writes a breakpoint global, this cannot: every global here is reached
/// through the `Store`, and `call` borrows it mutably for the whole execution — which is
/// exactly the window a stop has to work in. The interrupter is the mechanism Wasmer provides
/// for that, and it names only a `StoreId`, so it is reachable without the store.
///
/// It is also sharper than a global: the delivery is a signal, so a tight loop stops at once
/// instead of at the next injected checkpoint.
struct ExperimentalStopHandle {
    interrupter: Arc<Mutex<Interrupter>>,
}

// InstanceStopHandle requires Debug. Implemented by hand rather than derived, so that
// wasmer's Interrupter does not have to be Debug just to be held here.
impl std::fmt::Debug for ExperimentalStopHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExperimentalStopHandle").finish_non_exhaustive()
    }
}

impl InstanceStopHandle for ExperimentalStopHandle {
    fn request_stop(&self) {
        let Ok(interrupter) = self.interrupter.lock() else {
            return;
        };

        interrupter.interrupt();
    }
}
