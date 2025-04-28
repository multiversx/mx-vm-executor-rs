use crate::{BreakpointValue, ExecutorError};

/// The new instance trait, only used for configuring & calling the wasmer instance.
pub trait Instance {
    /// Calls an exported function of a WebAssembly instance by `name`.
    fn call(&self, func_name: &str) -> Result<(), ExecutorError>;

    /// Checks that all public module functions (SC endpoints) have no arguments or results.
    fn check_signatures(&self) -> bool;

    /// Checks whether SC has an endpoint with given name.
    fn has_function(&self, func_name: &str) -> bool;

    /// Required to be able to extract all SC endpoint names.
    fn get_exported_function_names(&self) -> Vec<String>;

    /// Sets the number of points(gas) limit for the given instance.
    fn set_points_limit(&self, limit: u64) -> Result<(), ExecutorError>;

    /// Returns the number of points(gas) used by the given instance.
    fn get_points_used(&self) -> Result<u64, ExecutorError>;

    /// Returns the runtime breakpoint value from the given instance.
    fn get_breakpoint_value(&self) -> Result<BreakpointValue, ExecutorError>;

    /// Resets an instance, cleaning memories and globals.
    fn reset(&self) -> Result<(), ExecutorError>;

    /// Caches an instance.
    fn cache(&self) -> Result<Vec<u8>, ExecutorError>;
}
