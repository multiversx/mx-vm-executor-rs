//! Wiring this crate's backend into the shared hook dispatch.
//!
//! The one place the Wasmer 6 instance state is named. Everything else about hook dispatch —
//! the context, its pool, the table, the generated `VMHooks` implementation — lives in
//! `c-api-common` and knows the state only as an erased pointer.

use multiversx_chain_vm_executor::VMHooks;
use multiversx_chain_vm_executor_c_api_common::capi_vm_hook_context::CapiVMHooksBuilder;
use multiversx_chain_vm_executor_c_api_common::capi_vm_hook_table::CapiVMHookTable;
use multiversx_chain_vm_executor_wasmer_experimental::{
    ExperimentalInstanceState, ExperimentalVMHooksBuilder,
};

/// The instance state type this backend erases into a hook context.
///
/// `with_instance_state` has to be told the type back, and this is the single definition both
/// sides go through, so the two cannot drift.
pub type BackendInstanceState = ExperimentalInstanceState<'static>;

/// Implements this backend's builder trait over the shared builder.
///
/// A newtype rather than an implementation on `CapiVMHooksBuilder` directly, because with the
/// builder in `c-api-common` both the trait and the type are foreign here and the orphan rule
/// forbids it. The wrapper costs nothing and makes the seam explicit.
pub struct BackendVMHooksBuilder(CapiVMHooksBuilder);

impl BackendVMHooksBuilder {
    pub fn new(go_vm_hooks: usize, hook_table: CapiVMHookTable) -> Self {
        BackendVMHooksBuilder(CapiVMHooksBuilder::new(go_vm_hooks, hook_table))
    }
}

impl ExperimentalVMHooksBuilder for BackendVMHooksBuilder {
    fn with_vm_hooks(
        &self,
        instance_state_ref: &mut ExperimentalInstanceState,
        f: &mut dyn FnMut(&mut dyn VMHooks),
    ) {
        self.0.with_vm_hooks_for_state(instance_state_ref, f);
    }
}
