//! C API for the new executor interface, over the **Wasmer 2.2** backend.
//!
//! The sibling of `c-api-wasmer6`, and deliberately as close to it as two libraries can be:
//! the three entry-point files are character-identical to that crate's apart from the symbol
//! prefix, and `tests/prefix_parity.rs` fails if they ever stop being. Everything behind them
//! — the operations, the hook dispatch, the error convention — is compiled once in
//! `c-api-common`. The only thing this crate decides is `capi_backend`.
//!
//! **Why it exists.** Wasmer 2.2 is the executor currently in production, and it is the
//! reference the new interfaces have to be measured against. Running the *same* runtime
//! through the *new* boundary is what isolates the boundary as the variable: a difference
//! between this library and `libvmexeccapi` is a difference the refactor introduced, with no
//! wasm runtime change to confound it. Comparing Wasmer 6 against Wasmer 2.2 cannot say that,
//! because it changes both at once.
//!
//! - **Exported symbols are prefixed `vm_exec_w2_`**, so this library, `libvmexeccapi_w6` and
//!   the legacy `libvmexeccapi` can all be linked into one binary.
//! - **Types are shared, not duplicated**: the vocabulary comes from `libvmexeccapi_common.h`,
//!   which both new headers include, so a caller linking both sees one definition of each.
//!
//! One thing this library does *not* expose, although the backend supports it: there is no
//! instance `reset`, and no `cache`. Those are the warm-instance path the legacy API has and
//! Wasmer 6 does not, and adding them here would make this library's shape depend on which
//! backend is underneath — which is the thing it exists to hold constant. A differential
//! against the legacy library therefore compares a warm cache against cold instances; the
//! hook sequence and the gas accounting, which are what it is measuring, are unaffected.

mod capi_logger;

pub mod capi_backend;
pub mod capi_executor;
pub mod capi_instance;
pub mod capi_instance_state;

// The shared C vocabulary, re-exported so that this crate's modules — and the cbindgen run
// that produces its header — reach the same definitions the common header publishes.
pub use multiversx_chain_vm_executor_c_api_common::{
    capi_call_result, capi_config, capi_executor_ops, capi_handles, capi_instance_ops,
    capi_opcode_cost, capi_state_ops, capi_status, capi_strings, capi_vm_hook_context,
    capi_vm_hook_pointers, capi_vm_hook_table, capi_vm_hooks,
};
