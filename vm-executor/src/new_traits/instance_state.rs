use crate::{ExecutorError, MemLength, MemPtr, MemoryError};

/// The interface through which VM hooks update the instance state.
///
/// All accesses are validated by the executor, which is the only side that knows the real
/// extent of the instance memory. The load and store methods are **strict**: a range that
/// does not fit entirely inside the memory is rejected with a [`MemoryError`]. The one
/// exception is [`InstanceState::memory_load_clamped`], which reproduces the Go VM's
/// historical truncating behaviour; see its documentation.
pub trait InstanceState {
    /// Returns the number of points(gas) used by the given instance.
    fn get_points_used(&mut self) -> Result<u64, ExecutorError>;

    /// Sets the number of points(gas) for the given instance.
    fn set_points_used(&mut self, points: u64) -> Result<(), ExecutorError>;

    /// Gets the size in bytes of the instance memory.
    fn memory_length(&self) -> Result<u64, ExecutorError>;

    /// Grows the instance memory by the given number of pages (of 64Kb each).
    ///
    /// Returns the number of pages the memory had before growing.
    fn memory_grow(&mut self, by_num_pages: u32) -> Result<u32, ExecutorError>;

    /// Loads data from instance memory into the given slice.
    ///
    /// The requested range must fit entirely inside the memory.
    fn memory_load_to_slice(&self, mem_ptr: MemPtr, dest: &mut [u8]) -> Result<(), ExecutorError>;

    /// Copies data from instance memory to a new owned buffer.
    ///
    /// The requested range must fit entirely inside the memory.
    fn memory_load_owned(
        &self,
        mem_ptr: MemPtr,
        mem_length: MemLength,
    ) -> Result<Vec<u8>, ExecutorError>;

    /// Writes to instance memory.
    ///
    /// The requested range must fit entirely inside the memory.
    fn memory_store(&self, mem_ptr: MemPtr, data: &[u8]) -> Result<(), ExecutorError>;

    /// Copies data from instance memory to a new owned buffer, truncating the range at the
    /// end of the memory instead of rejecting it.
    ///
    /// This reproduces, deliberately, the behaviour the Go VM has always had: a load whose
    /// offset is valid but whose end lies past the end of the memory yields the bytes that
    /// are available, and no error. Contracts have observed this, so the executor has to
    /// keep offering it, distinct from the strict [`InstanceState::memory_load_owned`].
    ///
    /// An offset outside the memory, or a negative length, is still an error.
    fn memory_load_clamped(
        &self,
        mem_ptr: MemPtr,
        mem_length: MemLength,
    ) -> Result<Vec<u8>, ExecutorError> {
        if mem_length == 0 {
            return Ok(Vec::new());
        }

        let memory_length = self.memory_length()?;

        // Checked in this order to match the Go VM: an out-of-range offset takes
        // precedence over a negative length.
        if mem_ptr < 0 || mem_ptr as u64 > memory_length {
            return Err(MemoryError::BadBounds.into());
        }
        if mem_length < 0 {
            return Err(MemoryError::NegativeLength.into());
        }

        // Computed as a minimum rather than as `mem_ptr + mem_length`, which can overflow
        // for a length a contract is free to choose.
        let available = memory_length - mem_ptr as u64;
        let to_load = std::cmp::min(mem_length as u64, available);

        self.memory_load_owned(mem_ptr, to_load as MemLength)
    }
}
