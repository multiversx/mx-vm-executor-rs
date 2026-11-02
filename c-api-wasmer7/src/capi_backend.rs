//! Which wasm runtime this library is for.
//!
//! The one file in this crate that names a backend, and the reason the three entry-point
//! files do not: they reach the backend only through this function, under a name that is the
//! same in every backend crate. That is what keeps them identical to the Wasmer 2.2 copy
//! apart from the symbol prefix — a property an integration test checks, so the two cannot
//! drift.

use multiversx_chain_vm_executor::Executor;
use multiversx_chain_vm_executor_wasmer_experimental::ExperimentalExecutor;
use std::sync::Arc;

use crate::capi_vm_hook_context::CapiVMHooksBuilder;

/// Wasmer 7, through the experimental backend.
pub fn new_backend_executor(runtime: Arc<CapiVMHooksBuilder>) -> Box<dyn Executor> {
    Box::new(ExperimentalExecutor::new(runtime))
}
