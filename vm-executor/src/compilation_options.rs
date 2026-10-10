/// Maximum number of locals allowed in a single wasm function.
///
/// Same limit as in the Go VM (`WASMLocalsLimit`).
// TODO: make this part of the compilation options
pub const WASM_LOCALS_LIMIT: u32 = 4000;

pub struct CompilationOptionsLegacy {
    pub gas_limit: u64,
    pub unmetered_locals: usize,
    pub max_memory_grow: usize,
    pub max_memory_grow_delta: usize,
    pub opcode_trace: bool,
    pub metering: bool,
    pub runtime_breakpoints: bool,
}
