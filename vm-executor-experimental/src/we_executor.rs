use crate::{ExperimentalInstance, ExperimentalVMHooksBuilder};
use multiversx_chain_vm_executor::{
    CompilationOptions, Executor, ExecutorError, Instance, OpcodeCost, check_missing_wasm,
};
use std::{fmt, sync::Arc};

pub trait ExperimentalExecutorRuntimeRef: Send + Sync {
    fn vm_hooks_builder(&self) -> Box<dyn ExperimentalVMHooksBuilder>;

    fn opcode_cost(&self) -> Arc<OpcodeCost>;
}

/// Executor implementation that produces wasmer instances with correctly injected VM hooks from runtime.
pub struct ExperimentalExecutor {
    runtime_ref: Box<dyn ExperimentalExecutorRuntimeRef>,
}

impl fmt::Debug for ExperimentalExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExperimentalExecutor").finish()
    }
}

impl ExperimentalExecutor {
    pub fn new(runtime_ref: Box<dyn ExperimentalExecutorRuntimeRef>) -> Self {
        ExperimentalExecutor { runtime_ref }
    }

    fn new_instance_from_bytes(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Box<dyn Instance> {
        Box::new(
            ExperimentalInstance::try_new_instance(
                self.runtime_ref.vm_hooks_builder(),
                self.runtime_ref.opcode_cost(),
                wasm_bytes,
                compilation_options,
            )
            .expect("instance init failed"),
        )
    }
}

impl Executor for ExperimentalExecutor {
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
