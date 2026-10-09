//! What the opaque C handles point to, and the one thing the two C APIs do not share.
//!
//! Both backends' handles hold exactly the same thing, because neither names a backend: an
//! executor is a configuration plus a hook table, and an instance is a `Box<dyn Instance>`.
//! The single point of variation is [`CapiBackend`] — which `Executor` implementation an
//! instance is built from — and it is a trait rather than a parameter so that the shared
//! operations stay plain functions a prefixed entry point can call directly.

use std::sync::Arc;

use multiversx_chain_vm_executor::{Executor, Instance, OpcodeConfig};

use crate::capi_vm_hook_context::CapiVMHooksBuilder;
use crate::capi_vm_hook_table::CapiVMHookTable;

/// What a `vm_exec_<prefix>_executor_t` points to.
///
/// The shared, immutable configuration: passed once at creation and never mutated. It does
/// not yet hold a `dyn Executor`, because an executor is built around one runtime reference
/// for its whole life and this API's runtime handle still arrives per instance. See
/// `capi_instance_ops::new_instance`.
pub struct CapiExecutor {
    pub opcode_config: Arc<OpcodeConfig>,
    pub hook_table: CapiVMHookTable,
}

/// What a `vm_exec_<prefix>_instance_t` points to.
pub struct CapiInstance {
    pub content: Box<dyn Instance>,
}

/// Which wasm runtime a C API is for.
///
/// This is the entire difference between the two libraries. Everything else — the handles,
/// the operations, the hook dispatch, the error convention — is compiled once and shared,
/// which is why adding a backend costs one implementation of this plus a file of prefixed
/// entry points.
pub trait CapiBackend {
    /// Builds the backend's executor around the runtime the C API supplies.
    fn new_executor(runtime: Arc<CapiVMHooksBuilder>) -> Box<dyn Executor>;
}
