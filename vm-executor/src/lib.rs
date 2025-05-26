mod breakpoint_value;
mod compilation_options;
mod executor;
mod instance;
mod missing_wasm;
mod new_traits;
mod opcode_cost;
mod service_error;
mod service_trait;
mod vm_hooks;

pub use breakpoint_value::*;
pub use compilation_options::CompilationOptionsLegacy;
pub use executor::ExecutorLegacy;
pub use instance::InstanceLegacy;
pub use missing_wasm::{check_missing_wasm, MissingWasmError};
pub use new_traits::*;
pub use opcode_cost::OpcodeCost;
pub use service_error::ServiceError;
pub use service_trait::*;
pub use vm_hooks::{VMHooksLegacy, VMHooksLegacyDefault};

/// The argument type for dealing with executor memory pointers.
pub type MemPtr = isize;

/// The argument type for dealing with lengths of slices of the executor memory.
pub type MemLength = isize;
