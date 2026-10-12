use crate::WasmerInstance;
use crate::executor_interface::{
    CompilationOptions, Executor, ExecutorError, ExecutorRuntime, Instance, OpcodeConfig,
    VMHooksLegacy, VMHooksLegacyAdapter, check_missing_wasm,
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

    /// Builds an instance from either source.
    ///
    /// The two paths differ only in how the `WasmerInstance` is constructed — compile, or
    /// deserialize — and everything around that is identical: the cyclic hook wiring and the
    /// wrapper. So `build` is the one thing passed in.
    fn new_instance_with(
        &self,
        build: impl FnOnce(Rc<dyn VMHooksLegacy>) -> Result<WasmerInstance, ExecutorError>,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
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

            build(Rc::new(vm_hooks))
        })?;

        Ok(Box::new(WasmerProdInstance::new(inner_instance_ref)))
    }
}

/// The 2.2 middleware wants the config behind a `Mutex`, which was how the gas schedule used
/// to be swapped under a live executor. Nothing mutates it any more: the config now arrives
/// per instance in the compilation options, so this is a private copy that no one else holds
/// a handle to.
fn private_opcode_config(compilation_options: &CompilationOptions) -> Arc<Mutex<OpcodeConfig>> {
    Arc::new(Mutex::new((*compilation_options.opcode_config).clone()))
}

impl Executor for WasmerProdExecutor {
    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        check_missing_wasm(wasm_bytes)?;

        let opcode_config = private_opcode_config(compilation_options);
        let legacy = compilation_options.to_legacy();
        self.new_instance_with(|vm_hooks| {
            WasmerInstance::try_new_instance(vm_hooks, opcode_config, wasm_bytes, &legacy)
        })
    }

    /// Instantiates a previously serialized module.
    ///
    /// Same trust note as the Wasmer 7 backend: `Module::deserialize` maps executable code
    /// without validating it, so artifacts must come from a store the host trusts. Here they
    /// come from [`Instance::cache`] on this same library.
    fn new_instance_from_cache(
        &self,
        cache_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        if cache_bytes.is_empty() {
            return Err(WasmerExecutorError::EmptyCacheArtifact.into());
        }

        let opcode_config = private_opcode_config(compilation_options);
        let legacy = compilation_options.to_legacy();
        self.new_instance_with(|vm_hooks| {
            WasmerInstance::try_new_instance_from_cache(
                vm_hooks,
                opcode_config,
                cache_bytes,
                &legacy,
            )
        })
    }
}
