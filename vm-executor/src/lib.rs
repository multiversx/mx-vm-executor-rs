mod breakpoint_value;
mod executor;
mod executor_new;
mod instance;
mod instance_new;
mod instance_state;
mod missing_wasm;
mod opcode_cost;
mod service_error;
mod service_trait;
mod vm_hooks;
mod vm_hooks_early_exit;
mod vm_hooks_legacy_adapter;
mod vm_hooks_new;

pub use breakpoint_value::*;
pub use executor::ExecutorLegacy;
pub use executor_new::Executor;
pub use instance::*;
pub use instance_new::{Instance, InstanceCallError};
pub use instance_state::InstanceState;
pub use missing_wasm::{check_missing_wasm, MissingWasmError};
pub use opcode_cost::OpcodeCost;
pub use service_error::ServiceError;
pub use service_trait::*;
pub use vm_hooks::{VMHooksLegacy, VMHooksLegacyDefault};
pub use vm_hooks_early_exit::VMHooksEarlyExit;
pub use vm_hooks_legacy_adapter::VMHooksLegacyAdapter;
pub use vm_hooks_new::{VMHooks, VMHooksDefault};

/// Temporary. TODO: delete.
pub type VMHooksError = VMHooksEarlyExit;
