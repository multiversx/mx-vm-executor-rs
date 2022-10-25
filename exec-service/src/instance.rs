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

    // metering
    fn get_points_used(&self) -> u64;
    fn set_points_used(&self, new_gas: u64);
    fn set_points_limit(&self, new_limit: u64);
}
