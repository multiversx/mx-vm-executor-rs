//! Satisfies the path `meta::capi_safe_unwind` expands to.
//!
//! The attribute emits `crate::capi_logger::set_panic_handler()`, so every crate that uses it
//! needs a module of that name at its own root. The implementation is shared; only the path
//! has to be local.
pub use multiversx_chain_vm_executor_c_api_common::set_panic_handler;
