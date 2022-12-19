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
    /// Calls an exported function of a WebAssembly instance by `name`.
    fn call(&self, func_name: &str) -> Result<(), String>;

    /// Checks that all public module functions (SC endpoints) have no arguments or results.
    fn check_signatures(&self) -> bool;

    /// Checks whether SC has an endpoint with given name.
    fn has_function(&self, func_name: &str) -> bool;

    /// Required to be able to extract all SC endpoint names.
    fn get_exported_function_names(&self) -> Vec<String>;

    /// Sets the number of points(gas) limit for the given instance.
    fn set_points_limit(&self, limit: u64);

    /// Sets the number of points(gas) for the given instance.
    fn set_points_used(&self, points: u64);

    /// Returns the number of points(gas) used by the given instance.
    fn get_points_used(&self) -> u64;

    /// Gets the size in bytes of the memory data.
    fn memory_length(&self) -> u64;

    /// Gets a pointer to the beginning of the contiguous memory data bytes.
    fn memory_ptr(&self) -> *mut u8;

    /// Grows a memory by the given number of pages (of 65Kb each).
    fn memory_grow(&self, by_num_pages: u32) -> Result<u32, ExecutorError>;

    /// Sets the runtime breakpoint value for the given instance.
    fn set_breakpoint_value(&self, value: u64);

    /// Returns the runtime breakpoint value from the given instance.
    fn get_breakpoint_value(&self) -> u64;

    /// Resets an instance, cleaning memories and globals.
    fn reset(&self) -> Result<(), String>;

    /// Caches an instance.
    fn cache(&self) -> Result<Vec<u8>, String>;
}
