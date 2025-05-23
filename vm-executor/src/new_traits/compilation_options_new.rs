use crate::CompilationOptionsLegacy;

pub struct CompilationOptions {
    pub unmetered_locals: usize,
    pub max_memory_grow: usize,
    pub max_memory_grow_delta: usize,
    pub opcode_trace: bool,
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
