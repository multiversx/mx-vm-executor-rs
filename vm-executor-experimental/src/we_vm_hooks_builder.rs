use multiversx_chain_vm_executor::{VMHooks, VMHooksDefault};

use crate::ExperimentalInstanceState;

/// Creates the VM hooks used to service one host function call.
///
/// The hooks borrow the instance state, which is only reachable for the duration of the
/// call, so they cannot outlive it and cannot be returned to the caller. Rather than
/// heap-allocating a `Box<dyn VMHooks>` on every single hook invocation, implementations
/// build the hooks on their own stack frame and lend them to the given closure.
pub trait ExperimentalVMHooksBuilder {
    /// Builds the VM hooks around the given instance state and passes them to `f`.
    ///
    /// Implementations must call `f` exactly once.
    fn with_vm_hooks(
        &self,
        instance_state_ref: &mut ExperimentalInstanceState,
        f: &mut dyn FnMut(&mut dyn VMHooks),
    );
}

#[derive(Debug)]
pub struct ExperimentalVMHooksBuilderDefault;

impl ExperimentalVMHooksBuilder for ExperimentalVMHooksBuilderDefault {
    fn with_vm_hooks(
        &self,
        _instance_state_ref: &mut ExperimentalInstanceState,
        f: &mut dyn FnMut(&mut dyn VMHooks),
    ) {
        f(&mut VMHooksDefault);
    }
}
