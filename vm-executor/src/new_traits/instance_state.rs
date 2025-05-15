use crate::{BreakpointValue, ExecutorError, MemLength, MemPtr};

/// The interface through which VM hooks update the instance state.
pub trait InstanceState {
    /// Gets the number of points(gas) limit for the given instance.
    fn get_points_limit(&self) -> Result<u64, ExecutorError>;

    /// Sets the number of points(gas) for the given instance.
    fn set_points_used(&mut self, points: u64) -> Result<(), ExecutorError>;

    /// Returns the number of points(gas) used by the given instance.
    fn get_points_used(&self) -> Result<u64, ExecutorError>;

    /// Copies data to new owned buffer.
    fn memory_load_to_slice(&self, mem_ptr: MemPtr, dest: &mut [u8]) -> Result<(), ExecutorError>;

    /// Copies data to new owned buffer.
    fn memory_load_owned(
        &self,
        mem_ptr: MemPtr,
        mem_length: MemLength,
    ) -> Result<Vec<u8>, ExecutorError>;

    /// Loads data to given slice. In certain cases
    fn memory_store(&self, mem_ptr: MemPtr, data: &[u8]) -> Result<(), ExecutorError>;

    /// Sets the runtime breakpoint value for the given instance.
    fn set_breakpoint_value(&mut self, value: BreakpointValue) -> Result<(), ExecutorError>;
}
