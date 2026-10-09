//! Everything the new C APIs share, independent of which wasm runtime is underneath.
//!
//! There are two C APIs built on the second executor interface set — one per backend, with
//! distinct symbol prefixes so both can be linked into one binary — and they are near
//! identical, because they are built on the same traits. What differs between them is only
//! the backend wiring: which `Executor` is instantiated, and what the instance state is.
//! Everything else lives here and is compiled once.
//!
//! What that covers:
//!
//! - **The shared C vocabulary**: status codes, the error-buffer convention, configuration,
//!   the opcode cost table. These are published under the neutral `vm_exec_*` prefix and
//!   emitted into a single `libvmexeccapi_common.h` that each backend's header includes, so a
//!   caller that links both libraries sees one definition of each rather than two.
//! - **Helpers with no ABI of their own**: the two-step string-out convention, panic
//!   suppression.
//! - **The whole hook dispatch path**: the result and context ABI, the per-instance context
//!   pool, the function-pointer table, and the generated `VMHooks` implementation that calls
//!   through it. Over 1,200 generated lines, compiled once instead of once per backend. None
//!   of it names a backend, because the context carries the instance state as an erased
//!   pointer and only the backend's own builder implementation ever names its type.
//!
//! - **The operations themselves**: executor and instance lifecycle, the outer instance
//!   calls, and the state calls a hook makes. Only one of them is parameterised by backend —
//!   which `Executor` an instance is built from, named by `capi_handles::CapiBackend`.
//!
//! The backend crates keep only their opaque handles and their prefixed entry points, which
//! are thin wrappers over the operations here. They have to be written out literally rather
//! than expanded from a macro, because cbindgen parses the source syntactically and generates
//! a header only for the entry points it can see; that is also why the C contract is
//! documented on the wrappers, where it becomes the header, and the rationale here. The
//! legacy C API is not part of this: its types were renamed to `vm_legacy_*` precisely so
//! that it can coexist with these without colliding, and it is on its way out.

mod capi_logger;

pub mod capi_call_result;
pub mod capi_config;
pub mod capi_executor_ops;
pub mod capi_handles;
pub mod capi_instance_ops;
pub mod capi_opcode_cost;
pub mod capi_state_ops;
pub mod capi_status;
pub mod capi_strings;
pub mod capi_vm_hook_context;
pub mod capi_vm_hook_pointers;
pub mod capi_vm_hook_table;
pub mod capi_vm_hooks;

pub use capi_logger::set_panic_handler;
