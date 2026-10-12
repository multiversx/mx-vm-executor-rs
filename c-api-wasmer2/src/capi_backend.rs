//! Which wasm runtime this library is for.
//!
//! The one file in this crate that names a backend, and the reason the three entry-point
//! files do not: they reach the backend only through this function, under a name that is the
//! same in every backend crate. That is what keeps them identical to the Wasmer 7 copy
//! apart from the symbol prefix — a property an integration test checks, so the two cannot
//! drift.

use multiversx_chain_vm_executor::Executor;
use multiversx_chain_vm_executor_wasmer::new_traits::WasmerProdExecutor;
use std::sync::Arc;

use crate::capi_vm_hook_context::CapiVMHooksBuilder;

/// Wasmer 2.2, through the production backend.
///
/// Its hooks are the legacy value-returning kind, with no channel for an early exit, so the
/// backend wraps the hooks this C API builds in `VMHooksLegacyAdapter`. Nothing about that is
/// visible here or to the caller: the adapter asks the runtime for hooks per call, exactly as
/// Wasmer 7 does, so both libraries resolve the current execution when a hook runs.
pub fn new_backend_executor(runtime: Arc<CapiVMHooksBuilder>) -> Box<dyn Executor> {
    Box::new(WasmerProdExecutor::new(runtime))
}
