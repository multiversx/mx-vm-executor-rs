//! FFI-safe mirror of `CompilationOptionsLegacy`.

use multiversx_chain_vm_executor::CompilationOptionsLegacy;

/// FFI-safe mirror of `CompilationOptionsLegacy`.
///
/// This is a distinct type rather than a cast of `CompilationOptionsLegacy`
/// itself, so that the general-purpose struct is free to change shape
/// without touching the C ABI, and vice versa. Fields that are `usize` on
/// the Rust side are fixed to `u64` here, since `usize` has no guaranteed
/// width across host platforms.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct vm_exec_compilation_options_t {
    pub gas_limit: u64,
    pub unmetered_locals: u64,
    pub max_memory_grow: u64,
    pub max_memory_grow_delta: u64,
    pub opcode_trace: bool,
    pub metering: bool,
    pub runtime_breakpoints: bool,
}

impl vm_exec_compilation_options_t {
    pub(crate) fn to_legacy(&self) -> CompilationOptionsLegacy {
        CompilationOptionsLegacy {
            gas_limit: self.gas_limit,
            unmetered_locals: self.unmetered_locals as usize,
            max_memory_grow: self.max_memory_grow as usize,
            max_memory_grow_delta: self.max_memory_grow_delta as usize,
            opcode_trace: self.opcode_trace,
            metering: self.metering,
            runtime_breakpoints: self.runtime_breakpoints,
        }
    }
}
