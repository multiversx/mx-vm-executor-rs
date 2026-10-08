use crate::{InstanceState, VMHooks, VMHooksDefault};

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
