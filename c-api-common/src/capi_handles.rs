//! What the opaque C handles point to.
//!
//! Both backends' handles hold exactly the same thing, because neither names a backend: an
//! executor is a configuration plus a hook table, and an instance is a `Box<dyn Instance>`.

use std::sync::Arc;

use multiversx_chain_vm_executor::{Executor, Instance, OpcodeConfig};

use crate::capi_vm_hook_context::CapiVMHooksBuilder;
use crate::capi_vm_hook_table::CapiVMHookTable;

/// What a `vm_exec_<prefix>_executor_t` points to.
///
/// The shared, immutable configuration: passed once at creation and never mutated. The
/// backend is part of that configuration, because the handle is already backend-specific —
/// `vm_exec_w2_executor_t` and `vm_exec_w7_executor_t` are distinct types — so an instance
/// call has nothing left to decide and is not asked to.
///
/// It does not yet hold a `dyn Executor`, and the reason is on the Go side rather than here.
/// An executor is built around one runtime reference for its whole life, and this API's
/// runtime handle is still supplied per instance — because the Go object it identifies holds
/// `activeState`, the state of the hook currently running *on that instance*, which a handle
/// shared by several instances could not hold. That slot exists only because the VM still
/// reaches wasm memory through `executor.Instance` instead of through the state it was
/// handed. When `MemLoad`/`MemStore` leave that interface (PLAN-go-ffi.md, Phase 4b), the
/// handle becomes per-executor, moves to `new_executor`, and the `Box<dyn Executor>` moves
/// here with it.
pub struct CapiExecutor {
    pub opcode_config: Arc<OpcodeConfig>,
    pub hook_table: CapiVMHookTable,

    /// Which backend this library is for, supplied by the entry point that created this.
    pub new_backend_executor: NewBackendExecutor,
}

/// What a `vm_exec_<prefix>_instance_t` points to.
pub struct CapiInstance {
    pub content: Box<dyn Instance>,
}

/// Builds the backend's executor around the runtime the C API supplies.
///
/// This is the entire difference between the two libraries, and a library has exactly one of
/// them: a C API is built for a single wasm runtime, fixed when the crate is compiled. So it
/// travels as an ordinary function, handed to [`crate::capi_executor_ops::new_executor`] and
/// kept on the [`CapiExecutor`] it builds. Everything else — the handles, the operations, the
/// hook dispatch, the error convention — is compiled once and shared.
pub type NewBackendExecutor = fn(Arc<CapiVMHooksBuilder>) -> Box<dyn Executor>;
