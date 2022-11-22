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
    fn set_points_limit(&self, limit: u64);
    fn set_points_used(&self, points: u64);
    fn get_points_used(&self) -> u64;
    fn memory_length(&self) -> u64;
    fn memory_ptr(&self) -> *mut u8;
    fn memory_grow(&self, by_num_pages: u32) -> Result<u32, ExecutorError>;
    fn set_breakpoint_value(&self, value: u64);
    fn get_breakpoint_value(&self) -> u64;
}
