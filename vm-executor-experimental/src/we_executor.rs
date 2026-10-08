use crate::{ExperimentalError, ExperimentalInstance};
use multiversx_chain_vm_executor::{
    CompilationOptions, Executor, ExecutorError, ExecutorRuntime, Instance, check_missing_wasm,
};
use std::{fmt, sync::Arc};

/// Executor implementation that produces wasmer instances with correctly injected VM hooks from runtime.
pub struct ExperimentalExecutor {
    runtime: Arc<dyn ExecutorRuntime>,
}

impl fmt::Debug for ExperimentalExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExperimentalExecutor").finish()
    }
}

impl ExperimentalExecutor {
    pub fn new(runtime: Arc<dyn ExecutorRuntime>) -> Self {
        ExperimentalExecutor { runtime }
    }
}

impl Executor for ExperimentalExecutor {
    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        check_missing_wasm(wasm_bytes)?;

        let instance = ExperimentalInstance::try_new_instance(
            self.runtime.clone(),
            wasm_bytes,
            compilation_options,
        )?;

        Ok(Box::new(instance))
    }

    fn new_instance_from_cache(
        &self,
        _cache_bytes: &[u8],
        _compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        Err(ExperimentalError::InstanceFromCacheUnsupported.into())
    }
}
