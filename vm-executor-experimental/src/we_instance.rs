#![allow(unused)]

use crate::middlewares::{
    get_breakpoint_value, get_points_used, set_points_limit, Breakpoints, Metering, OpcodeControl,
    OpcodeTracer, ProtectedGlobals,
};
use crate::we_instance_state::ExperimentalInstanceState;
use crate::{we_imports::generate_import_object, we_vm_hooks::VMHooksWrapper};
use log::trace;
use multiversx_chain_vm_executor::{
    BreakpointValue, CompilationOptions, ExecutorError, Instance, InstanceFull, InstanceState,
    OpcodeCost, ServiceError, VMHooksBuilder,
};
use multiversx_chain_vm_executor::{MemLength, MemPtr};

use std::cell::RefCell;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::rc::Weak;
use std::{rc::Rc, sync::Arc};
use wasmer::{
    imports, AsStoreMut, CompilerConfig, Extern, Memory, Module, Pages, Singlepass, Store, StoreMut,
};

const MAX_MEMORY_PAGES_ALLOWED: Pages = Pages(20);

pub struct ExperimentalInstance {
    wasmer_store: RefCell<wasmer::Store>,
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
    vm_hooks_builder: Rc<dyn VMHooksBuilder>,
    module: &Module,
    store: &mut Store,
    weak: Weak<ExperimentalInstanceInner>,
) -> Result<ExperimentalInstanceInner, ExecutorError> {
    // Create an empty import object.
    trace!("Generating imports ...");
    let vm_hooks_wrapper = VMHooksWrapper {
        vm_hooks_builder,
        wasmer_inner: weak,
    };
    let import_object = generate_import_object(store, vm_hooks_wrapper);

    trace!("Instantiating WasmerInstance ...");
    let wasmer_instance = wasmer::Instance::new(store, module, &import_object)?;
    // set_points_limit(&wasmer_instance, compilation_options.gas_limit)?;

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

fn new_cyclic_fallible<T, E, F>(f: F) -> Result<Rc<T>, E>
where
    F: FnOnce(Weak<T>) -> Result<T, E>,
{
    let mut result: Result<(), E> = Ok(());
    let maybe_uninit_rc = Rc::<MaybeUninit<T>>::new_cyclic(|weak_uninit| unsafe {
        let raw = Weak::into_raw(weak_uninit.clone());
        let weak = Weak::<T>::from_raw(raw as *const T);
        match f(weak) {
            Ok(t) => MaybeUninit::<T>::new(t),
            Err(err) => {
                result = Err(err);
                MaybeUninit::<T>::uninit()
            }
        }
    });
    result?;
    let raw = Rc::into_raw(maybe_uninit_rc);
    unsafe { Ok(Rc::from_raw(raw as *const T)) }
}

impl ExperimentalInstance {
    pub fn try_new_instance(
        vm_hooks_builder: Rc<dyn VMHooksBuilder>,
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

        let inner = new_cyclic_fallible(|weak| {
            prepare_wasmer_instance_inner(vm_hooks_builder.clone(), &module, &mut store, weak)
        })?;

        Ok(ExperimentalInstance {
            wasmer_store: RefCell::new(store),
            inner,
        })
    }
}

fn get_memory_ref<'a>(
    wasmer_instance: &'a wasmer::Instance,
    memory_name: &'a str,
) -> Result<&'a wasmer::Memory, String> {
    let result = wasmer_instance.exports.get_memory(memory_name);
    match result {
        Ok(memory) => Ok(memory),
        Err(err) => Err(err.to_string()),
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
            max_memory_pages,
            MAX_MEMORY_PAGES_ALLOWED
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
        compilation_options.gas_limit,
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
    fn call(&self, func_name: &str) -> Result<(), String> {
        trace!("Rust instance call: {func_name}");

        let func = self
            .inner
            .wasmer_instance
            .exports
            .get_function(func_name)
            .map_err(|_| "function not found".to_string())?;

        match func.call(&mut *self.wasmer_store.borrow_mut(), &[]) {
            Ok(_) => {
                trace!("Call succeeded: {func_name}");
                Ok(())
            }
            Err(err) => {
                trace!("Call failed: {func_name} - {err}");
                Err(err.to_string())
            }
        }
    }

    fn check_signatures(&self) -> bool {
        for (_, export) in self.inner.wasmer_instance.exports.iter() {
            if let Extern::Function(endpoint) = export {
                if endpoint.param_arity(&self.wasmer_store.borrow()) > 0 {
                    return false;
                }
                if endpoint.result_arity(&self.wasmer_store.borrow()) > 0 {
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

    fn set_points_limit(&self, limit: u64) -> Result<(), String> {
        set_points_limit(
            &self.inner.wasmer_instance,
            &mut self.wasmer_store.borrow_mut(),
            limit,
        )
    }

    fn get_points_used(&self) -> Result<u64, String> {
        get_points_used(
            &self.inner.wasmer_instance,
            &mut self.wasmer_store.borrow_mut(),
        )
    }

    fn get_breakpoint_value(&self) -> Result<BreakpointValue, String> {
        get_breakpoint_value(
            &self.inner.wasmer_instance,
            &mut self.wasmer_store.borrow_mut(),
        )?
        .try_into()
    }

    fn reset(&self) -> Result<(), String> {
        panic!("ExperimentalInstance reset not supported")
    }

    fn cache(&self) -> Result<Vec<u8>, String> {
        let module = self.inner.wasmer_instance.module();
        match module.serialize() {
            Ok(bytes) => Ok(bytes.to_vec()),
            Err(err) => Err(err.to_string()),
        }
    }
}
