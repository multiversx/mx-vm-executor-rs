use std::sync::Arc;

use crate::{CompilationOptionsLegacy, OpcodeConfig};

/// Everything that determines how a wasm module is compiled.
///
/// Passed to [`crate::Executor::new_instance`] on every instance, rather than configured on
/// the executor: these are essential execution logic, identical for every instance the host
/// cares about, and a compiled module is only interchangeable with another compiled from the
/// same options. Keeping them here means the pair (wasm, options) fully determines the
/// result, which is what lets a module cache be keyed on it.
///
/// That includes the opcode costs, which used to sit on the executor behind a mutable setter.
/// A gas schedule change is therefore a new set of options — and, for a host that caches,
/// a different cache key — rather than a reconfigured executor.
#[derive(Clone, Debug, Default)]
pub struct CompilationOptions {
    pub unmetered_locals: usize,
    pub max_memory_grow: usize,
    pub max_memory_grow_delta: usize,
    pub opcode_trace: bool,

    /// The opcode set and the per-opcode gas costs the metering middleware is built with.
    ///
    /// Shared rather than owned: the cost table is some hundreds of bytes and every instance
    /// under one gas schedule uses the same one, so the host builds it once and the
    /// middleware keeps this handle.
    pub opcode_config: Arc<OpcodeConfig>,
}

impl CompilationOptions {
    pub fn to_legacy(&self) -> CompilationOptionsLegacy {
        CompilationOptionsLegacy {
            gas_limit: 0,
            unmetered_locals: self.unmetered_locals,
            max_memory_grow: self.max_memory_grow,
            max_memory_grow_delta: self.max_memory_grow_delta,
            opcode_trace: self.opcode_trace,
            metering: true,
            runtime_breakpoints: true,
        }
    }
}
