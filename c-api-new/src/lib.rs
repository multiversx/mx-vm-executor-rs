//! C API for the new executor interface (`vm-executor/src/new_traits`).
//!
//! Built alongside the legacy `c-api` rather than replacing it, so the wasmer2 path keeps
//! shipping while this one is brought up. The two differ in four ways that matter to a
//! caller:
//!
//! - **Exported symbols are prefixed `vm_exec2_`**, so both libraries can be linked into one
//!   binary. The legacy API already owns `vm_exec_new_executor` and friends.
//! - **Errors travel through a caller-provided buffer**, not a thread-local last-error
//!   singleton. See `capi_status`.
//! - **Configuration is passed once**, at the point the thing it configures is created.
//!   There are no post-construction setters.
//! - **A call reports what happened in its result**, instead of leaving the caller to poll a
//!   wasm global afterwards. See `vm_exec2_call_result_t`.
//!
//! The hook dispatch itself — the typed function-pointer table and the 277-method `VMHooks`
//! implementation that calls into it — is generated code and arrives with Phase 3 of
//! `PLAN-go-ffi.md`. Everything it needs is here: the result and context ABI
//! (`capi_hook_context`), the table ownership (`capi_hook_table`) and the state operations
//! (`capi_instance_state`).

mod capi_logger;

pub mod capi_config;
pub mod capi_executor;
pub mod capi_hook_context;
pub mod capi_hook_table;
pub mod capi_instance;
pub mod capi_instance_state;
pub mod capi_opcode_cost;
pub mod capi_status;
pub(crate) mod capi_strings;
