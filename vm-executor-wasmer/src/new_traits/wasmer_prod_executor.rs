use crate::WasmerInstance;
use crate::executor_interface::{
    CompilationOptions, Executor, ExecutorError, Instance, OpcodeCost, VMHooksLegacy,
    check_missing_wasm,
};
use std::{
    fmt,
    rc::Rc,
    sync::{Arc, Mutex},
};

use super::{WasmerProdInstance, WasmerProdInstanceState};

pub trait WasmerProdRuntimeRef: Send + Sync {
    fn vm_hooks(&self, instance_state: WasmerProdInstanceState) -> Box<dyn VMHooksLegacy>;

    fn opcode_cost(&self) -> Arc<Mutex<OpcodeCost>>;
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
    ) -> Box<dyn Instance> {
        let inner_instance_ref = Rc::new_cyclic(|weak| {
            let instance_state = WasmerProdInstanceState::new(weak.clone());
            let vm_hooks = self.runtime_ref.vm_hooks(instance_state);

            WasmerInstance::try_new_instance(
                Rc::from(vm_hooks),
                self.runtime_ref.opcode_cost(),
                wasm_bytes,
                &compilation_options.to_legacy(),
            )
            .expect("instance init failed")
        });

        let wasmer_instance_ref = WasmerProdInstance::new(inner_instance_ref);

        Box::new(wasmer_instance_ref)
    }
}

impl Executor for WasmerProdExecutor {
    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        check_missing_wasm(wasm_bytes)?;

        Ok(self.new_instance_from_bytes(wasm_bytes, compilation_options))
    }

    fn new_instance_from_cache(
        &self,
        _cache_bytes: &[u8],
        _compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        panic!("WasmerProdExecutor new_instance_from_cache not supported")
    }
}
