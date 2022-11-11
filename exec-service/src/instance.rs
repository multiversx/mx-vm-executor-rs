use crate::ExecutorError;

pub struct CompilationOptions {
    pub gas_limit: u64,
    pub unmetered_locals: usize,
    pub max_memory_grow: usize,
    pub max_memory_grow_delta: usize,
    pub opcode_trace: bool,
    pub metering: bool,
    pub runtime_breakpoints: bool,
}

pub trait Instance {
    fn call(&self, func_name: &str) -> Result<(), String>;

    fn check_signatures(&self) -> bool;

    fn has_function(&self, func_name: &str) -> bool;

    fn get_exported_function_names(&self) -> Vec<String>;

    fn memory_length(&self) -> u64;

    fn memory_ptr(&self) -> *mut u8;

    fn memory_grow(&self, by_num_pages: u32) -> Result<u32, ExecutorError>;
}
