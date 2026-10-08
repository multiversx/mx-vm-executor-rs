use std::sync::Arc;

use crate::{BreakpointValueLegacy, InstanceState, VMHooks, VMHooksDefault, VMHooksEarlyExit};

/// The host behind an executor: one reference, for the executor's whole life.
///
/// This is the only thing an executor needs from its host that is not in the wasm and not in
/// the compilation options. It is supplied once, at executor construction, and never
/// replaced — which is what lets an executor be configured at once and then treated as
/// immutable.
pub trait ExecutorRuntime: Send + Sync {
    /// Builds the VM hooks servicing one host call, around the instance state for that call,
    /// and lends them to `f` — which implementations must call exactly once.
    ///
    /// **Built per call, not per instance.** Under Wasmer 6 the instance state is only
    /// reachable for the duration of a call — memory and globals live behind the `StoreMut`
    /// the host function is handed — so hooks that hold it cannot outlive the call and
    /// cannot be returned to the caller. Lending rather than returning also avoids a
    /// `Box<dyn VMHooks>` allocation on every single hook invocation.
    ///
    /// **Which execution the hooks serve is resolved here**, by the host, from its own call
    /// stack — not captured when the instance was created. That is the property that makes a
    /// cached instance safe to reuse across transactions: the instance contributes the
    /// state, never the identity.
    ///
    /// The state arrives as `&mut dyn InstanceState` so that one runtime serves every
    /// backend.
    fn with_vm_hooks(
        &self,
        instance_state: &mut dyn InstanceState,
        f: &mut dyn FnMut(&mut dyn VMHooks),
    );

    /// Which breakpoint should halt the instance when a hook signals an early exit.
    ///
    /// **Only the legacy Wasmer 2.2 backend asks.** Its hooks return plain values, with no
    /// channel for an early exit, so the executor has to stash the exit and trip the
    /// instance's breakpoint global to stop it. Wasmer 6 hooks return the early exit directly
    /// and never call this.
    ///
    /// Stashing is the executor's job; only the mapping is the host's, and it has to be,
    /// because the code inside a [`VMHooksEarlyExit`] is the host VM's own return code and
    /// means nothing to the executor. A host whose hooks already set the breakpoint
    /// themselves — the Go VM does — wants the default and implements nothing.
    ///
    /// Defaulted rather than split into a second, legacy-only trait so that one `impl` serves
    /// both backends. A separate trait would be foreign to `c-api-common`, where the shared
    /// runtime lives, and the orphan rule would force a newtype back into every backend crate.
    fn early_exit_breakpoint(&self, _early_exit: &VMHooksEarlyExit) -> BreakpointValueLegacy {
        BreakpointValueLegacy::ExecutionFailed
    }
}

/// A shared handle to a runtime is itself a runtime.
///
/// Every executor holds its runtime as `Arc<dyn ExecutorRuntime>`, because the host supplies
/// it once and several instances share it. This impl is what lets that handle be passed on
/// to code generic over `R: ExecutorRuntime` — [`crate::VMHooksLegacyAdapter`] is the one
/// that needs it — without each backend writing a forwarding newtype.
///
/// `?Sized` so that the trait-object form, which is the only form anyone actually holds, is
/// covered.
impl<T: ExecutorRuntime + ?Sized> ExecutorRuntime for Arc<T> {
    fn with_vm_hooks(
        &self,
        instance_state: &mut dyn InstanceState,
        f: &mut dyn FnMut(&mut dyn VMHooks),
    ) {
        (**self).with_vm_hooks(instance_state, f)
    }

    fn early_exit_breakpoint(&self, early_exit: &VMHooksEarlyExit) -> BreakpointValueLegacy {
        (**self).early_exit_breakpoint(early_exit)
    }
}

/// Bridges a `with_vm_hooks`-style lender to a caller that has an `FnOnce` and wants its
/// result back.
///
/// The lending side takes an `FnMut`, because [`ExecutorRuntime`] has to stay object-safe,
/// while servicing a single hook is naturally an `FnOnce` returning a value. Every caller of
/// a runtime needs the same bridge, so it lives here rather than in each of them.
///
/// Panics if the lender breaks the contract of calling the closure exactly once.
pub fn with_vm_hooks_once<R>(
    lend: impl FnOnce(&mut dyn FnMut(&mut dyn VMHooks)),
    f: impl FnOnce(&mut dyn VMHooks) -> R,
) -> R {
    let mut f = Some(f);
    let mut result = None;

    lend(&mut |vm_hooks| {
        let f = f.take().expect("executor runtime called the closure twice");
        result = Some(f(vm_hooks));
    });

    result.expect("executor runtime did not call the closure")
}

/// A runtime that yields hooks doing nothing, for instances that never call into the host.
#[derive(Debug)]
pub struct ExecutorRuntimeDefault;

impl ExecutorRuntime for ExecutorRuntimeDefault {
    fn with_vm_hooks(
        &self,
        _instance_state: &mut dyn InstanceState,
        f: &mut dyn FnMut(&mut dyn VMHooks),
    ) {
        f(&mut VMHooksDefault);
    }
}
