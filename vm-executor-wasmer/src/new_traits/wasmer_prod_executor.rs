use crate::WasmerInstance;
use crate::executor_interface::{
    CompilationOptions, Executor, ExecutorError, ExecutorRuntime, Instance, VMHooksLegacyAdapter,
    check_missing_wasm,
};
use rc_new_cyclic_fallible::rc_new_cyclic_fallible;
use std::{
    fmt,
    rc::Rc,
    sync::{Arc, Mutex},
};

use super::{WasmerExecutorError, WasmerProdInstance, WasmerProdInstanceState};

/// Executor implementation that produces wasmer instances with correctly injected VM hooks from runtime.
pub struct WasmerProdExecutor {
    runtime: Arc<dyn ExecutorRuntime>,
}

impl fmt::Debug for WasmerProdExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmerProdExecutor").finish()
    }
}

impl WasmerProdExecutor {
    pub fn new(runtime: Arc<dyn ExecutorRuntime>) -> Self {
        WasmerProdExecutor { runtime }
    }

    fn new_instance_from_bytes(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        // The 2.2 middleware wants the config behind a `Mutex`, which was how the gas
        // schedule used to be swapped under a live executor. Nothing mutates it any more:
        // the config now arrives per instance in the compilation options, so this is a
        // private copy that no one else holds a handle to.
        let opcode_config = Arc::new(Mutex::new((*compilation_options.opcode_config).clone()));

        let inner_instance_ref = rc_new_cyclic_fallible(|weak| {
            // The legacy import object wants a hooks object for the instance's life, while
            // the runtime lends hooks per call; the adapter is the join. The hooks an
            // instance calls are therefore built fresh each time, around this instance's
            // state, so they serve whichever execution the host has current — never the one
            // that happened to be current when the instance was created.
            let vm_hooks = VMHooksLegacyAdapter::new(
                self.runtime.clone(),
                WasmerProdInstanceState::new(weak.clone()),
            );

            WasmerInstance::try_new_instance(
                Rc::new(vm_hooks),
                opcode_config.clone(),
                wasm_bytes,
                &compilation_options.to_legacy(),
            )
        })?;

        let wasmer_instance_ref = WasmerProdInstance::new(inner_instance_ref);

        Ok(Box::new(wasmer_instance_ref))
    }
}

impl Executor for WasmerProdExecutor {
    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        check_missing_wasm(wasm_bytes)?;

        self.new_instance_from_bytes(wasm_bytes, compilation_options)
    }

    fn new_instance_from_cache(
        &self,
        _cache_bytes: &[u8],
        _compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        Err(WasmerExecutorError::InstanceFromCacheUnsupported.into())
    }
}
