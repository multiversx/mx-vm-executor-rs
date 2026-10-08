//! C API for the new executor interface (`vm-executor/src/new_traits`).
//!
//! Built alongside the legacy `c-api` rather than replacing it, so the wasmer2 path keeps
//! shipping while this one is brought up. The two differ in four ways that matter to a
//! caller:
//!
//! - **Exported symbols are prefixed `vm_exec_w6_`**, so both libraries can be linked into one
//!   binary. The legacy API already owns `vm_exec_new_executor` and friends.
//! - **Errors travel through a caller-provided buffer**, not a thread-local last-error
//!   singleton. See `capi_status`.
//! - **Configuration is passed once**, at the point the thing it configures is created.
//!   There are no post-construction setters.
//! - **A call reports what happened in its result**, instead of leaving the caller to poll a
//!   wasm global afterwards. See `vm_exec_call_result_t`.
//!
//! The hook dispatch — the typed function-pointer table (`capi_vm_hook_pointers`) and the
//! `VMHooks` implementation that calls into it (`capi_vm_hooks`) — is generated from the Go
//! VM's hook definitions by the generator in `mx-chain-vm-go/vmhost/vmhooks/generate`. It
//! builds on the result and context ABI in `capi_vm_hook_context`, the table ownership in
//! `capi_vm_hook_table`, and the state operations in `capi_instance_state`.

mod capi_logger;

pub mod capi_executor;
pub mod capi_instance;
pub mod capi_instance_state;

// The shared C vocabulary, re-exported so that this crate's modules — and the cbindgen run
// that produces its header — reach the same definitions the common header publishes.
pub use multiversx_chain_vm_executor_c_api_common::{
    capi_call_result, capi_config, capi_opcode_cost, capi_status, capi_strings,
    capi_vm_hook_context, capi_vm_hook_pointers, capi_vm_hook_table, capi_vm_hooks,
};
