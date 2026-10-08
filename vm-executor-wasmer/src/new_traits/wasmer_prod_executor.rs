use crate::WasmerInstance;
use crate::executor_interface::{
    CompilationOptions, Executor, ExecutorError, Instance, VMHooksLegacy, check_missing_wasm,
};
use rc_new_cyclic_fallible::rc_new_cyclic_fallible;
use std::{
    fmt,
    rc::Rc,
    sync::{Arc, Mutex},
};

use super::{WasmerExecutorError, WasmerProdInstance, WasmerProdInstanceState};

/// The host behind a Wasmer 2.2 executor.
///
/// This is deliberately **not** [`multiversx_chain_vm_executor::ExecutorRuntime`], and the
/// difference is in the shape of the legacy hooks rather than in the design. `ExecutorRuntime`
/// *lends* hooks per host call; the legacy `VMHooksLegacy` is an **object**, built once and
/// installed in the import object for the instance's life, because its methods return plain
/// values and have nowhere to report an early exit — the hooks must therefore own a place to
/// stash one. So the runtime hands over a built object here, and the execution it serves is
/// whatever was current when the instance was created.
///
/// The practical consequence is that instances from this backend **cannot be reused across
/// executions**, which is exactly what Wasmer 6 is being converged for and why that backend
/// gets the lending trait. Closing the gap would mean regenerating `VMHooksLegacyAdapter`
/// around a lender and giving the early-exit slot a home outside the hooks; it is not worth
/// doing to a backend that is scheduled for deletion, and whose value until then is being an
/// unchanged reference for the differential tests.
pub trait WasmerProdRuntimeRef: Send + Sync {
    fn vm_hooks(&self, instance_state: WasmerProdInstanceState) -> Box<dyn VMHooksLegacy>;
}

/// Executor implementation that produces wasmer instances with correctly injected VM hooks from runtime.
pub struct WasmerProdExecutor {
    runtime_ref: Box<dyn WasmerProdRuntimeRef>,
}

impl fmt::Debug for WasmerProdExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WasmerProdExecutor").finish()
    }
}

impl WasmerProdExecutor {
    pub fn new(runtime_ref: Box<dyn WasmerProdRuntimeRef>) -> Self {
        WasmerProdExecutor { runtime_ref }
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
            let instance_state = WasmerProdInstanceState::new(weak.clone());
            let vm_hooks = self.runtime_ref.vm_hooks(instance_state);

            WasmerInstance::try_new_instance(
                Rc::from(vm_hooks),
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
