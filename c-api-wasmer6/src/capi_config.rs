//! FFI-safe mirrors of the configuration structs.
//!
//! Both are passed by pointer, once, at the point the thing they configure is created.
//! There are no post-construction setters in this API: the legacy
//! `vm_exec_executor_set_vm_hooks_ptr` / `vm_exec_set_opcode_config` pair does not come
//! along.

use multiversx_chain_vm_executor::{CompilationOptions, OpcodeConfig, OpcodeVersion};

use crate::capi_opcode_cost::vm_exec_opcode_cost_t;

/// Everything an executor is configured with.
///
/// The opcode cost table is referenced rather than inlined, because it is large and the
/// caller already holds one. It is read during the call and not retained.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct vm_exec_w6_executor_config_t {
    /// Which opcode set and whitelist to apply; see `OpcodeVersion` on the Rust side.
    pub opcode_version: i32,

    /// The per-opcode gas costs. Must not be null.
    pub opcode_cost: *const vm_exec_opcode_cost_t,
}

impl vm_exec_w6_executor_config_t {
    /// Converts to the executor's own config type.
    ///
    /// # Safety
    ///
    /// `self.opcode_cost` must be a valid, non-null pointer for the duration of the call.
    pub(crate) unsafe fn to_opcode_config(&self) -> Result<OpcodeConfig, String> {
        let Some(opcode_version) = OpcodeVersion::from_i32(self.opcode_version) else {
            return Err(format!(
                "invalid opcode version code: {}",
                self.opcode_version
            ));
        };

        if self.opcode_cost.is_null() {
            return Err("opcode cost ptr is null".to_string());
        }

        Ok(OpcodeConfig {
            opcode_version,
            opcode_cost: unsafe { (*self.opcode_cost).to_opcode_cost() },
        })
    }
}

/// FFI-safe mirror of the new `CompilationOptions`.
///
/// Four fields, against the legacy seven: `gas_limit` moved to the call, and metering and
/// runtime breakpoints are no longer optional. Fields that are `usize` on the Rust side are
/// fixed to `u64` here, since `usize` has no guaranteed width across host platforms.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct vm_exec_w6_compilation_options_t {
    pub unmetered_locals: u64,
    pub max_memory_grow: u64,
    pub max_memory_grow_delta: u64,
    pub opcode_trace: bool,
}

impl vm_exec_w6_compilation_options_t {
    pub(crate) fn to_compilation_options(&self) -> CompilationOptions {
        CompilationOptions {
            unmetered_locals: self.unmetered_locals as usize,
            max_memory_grow: self.max_memory_grow as usize,
            max_memory_grow_delta: self.max_memory_grow_delta as usize,
            opcode_trace: self.opcode_trace,
        }
    }
}
