use crate::{BreakpointValueLegacy, ExecutorError, MemLength, MemPtr};

/// The old instance trait, used both:
/// - from the "outside": configuring & calling the instance;
/// - from the "inside": by VMHooks, to update progress.
pub trait InstanceLegacy {
    /// Calls an exported function of a WebAssembly instance by `name`.
    fn call(&self, func_name: &str) -> Result<(), String>;

    /// Checks that all public module functions (SC endpoints) have no arguments or results.
    fn check_signatures(&self) -> bool;

    /// Checks whether SC has an endpoint with given name.
    fn has_function(&self, func_name: &str) -> bool;

    /// Checks whether SC has an imported function with given name.
    fn has_imported_function(&self, func_name: &str) -> bool;

    /// Required to be able to extract all SC endpoint names.
    fn get_exported_function_names(&self) -> Vec<String>;

    /// Sets the number of points(gas) limit for the given instance.
    fn set_points_limit(&self, limit: u64) -> Result<(), String>;

    /// Sets the number of points(gas) for the given instance.
    fn set_points_used(&self, points: u64) -> Result<(), String>;

    /// Returns the number of points(gas) used by the given instance.
    fn get_points_used(&self) -> Result<u64, String>;

    /// Gets the size in bytes of the memory data.
    fn memory_length(&self) -> Result<u64, String>;

    /// Gets a pointer to the beginning of the contiguous memory data bytes.
    fn memory_ptr(&self) -> Result<*mut u8, String>;

    /// Loads data from executor memory.
    fn memory_load(&self, mem_ptr: MemPtr, mem_length: MemLength) -> Result<&[u8], ExecutorError>;

    /// Loads data from executor memory.
    fn memory_store(&self, mem_ptr: MemPtr, data: &[u8]) -> Result<(), ExecutorError>;

    /// Grows a memory by the given number of pages (of 65Kb each).
    fn memory_grow(&self, by_num_pages: u32) -> Result<u32, ExecutorError>;

    /// Sets the runtime breakpoint value for the given instance.
    fn set_breakpoint_value(&self, value: BreakpointValueLegacy) -> Result<(), String>;

    /// Returns the runtime breakpoint value from the given instance.
    fn get_breakpoint_value(&self) -> Result<BreakpointValueLegacy, String>;

    /// Resets an instance, cleaning memories and globals.
    fn reset(&self) -> Result<(), String>;

    /// Caches an instance.
    fn cache(&self) -> Result<Vec<u8>, String>;
}
