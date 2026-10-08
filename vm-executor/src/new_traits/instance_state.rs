use crate::{
    BreakpointValueLegacy, ExecutorError, MemLength, MemPtr, ServiceError, VMHooksEarlyExit,
    clamped_load_len,
};

/// The interface through which VM hooks update the instance state.
///
/// All accesses are validated by the executor, which is the only side that knows the real
/// extent of the instance memory. The load and store methods are **strict**: a range that
/// does not fit entirely inside the memory is rejected with a [`crate::MemoryError`]. The one
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

        let to_load = clamped_load_len(mem_ptr, mem_length, self.memory_length()?)?;

        self.memory_load_owned(mem_ptr, to_load as MemLength)
    }

    /// Halts the instance on an early exit raised by a *legacy* hook, and stashes the exit
    /// where the caller will find it once the call unwinds.
    ///
    /// **Only the Wasmer 2.2 backend implements this**, and only because its hooks return
    /// plain values: a [`crate::VMHooksLegacy`] method has no channel for an early exit, so
    /// the exit has to be put somewhere and the instance stopped by tripping its breakpoint
    /// global. Wasmer 6 hooks return the early exit directly and never come through here.
    ///
    /// Both halves belong to the instance — it owns the stash and it owns the global — which
    /// is why they are one method on the state rather than two capabilities a backend has to
    /// surface separately. Which breakpoint, though, is the host's call: the code carried by
    /// an early exit is the host VM's own return code and means nothing to the executor, so
    /// it arrives already mapped, from
    /// [`crate::ExecutorRuntime::early_exit_breakpoint`].
    ///
    /// The default is a failure, because for every other backend reaching this point means
    /// the early exit is about to be silently dropped.
    fn handle_early_exit_legacy(
        &mut self,
        _early_exit: VMHooksEarlyExit,
        _breakpoint: BreakpointValueLegacy,
    ) -> Result<(), ExecutorError> {
        Err(Box::new(ServiceError::new(
            "early exit via breakpoint is only supported by the legacy Wasmer 2.2 backend",
        )))
    }
}

/// A mutable reference to an instance state is itself an instance state.
///
/// This is what lets [`crate::ExecutorRuntime::with_vm_hooks`] hand the state over as
/// `&mut dyn InstanceState` and have the host pass it straight on to code that is generic
/// over `S: InstanceState`. Without it, every host would need its own forwarding wrapper, and
/// the backend's concrete state type would leak back into the host — which is the thing the
/// erased state exists to prevent.
///
/// `?Sized` so that `&mut dyn InstanceState` is covered, not merely `&mut ConcreteState`.
impl<T: InstanceState + ?Sized> InstanceState for &mut T {
    fn get_points_used(&mut self) -> Result<u64, ExecutorError> {
        (**self).get_points_used()
    }

    fn set_points_used(&mut self, points: u64) -> Result<(), ExecutorError> {
        (**self).set_points_used(points)
    }

    fn memory_length(&self) -> Result<u64, ExecutorError> {
        (**self).memory_length()
    }

    fn memory_grow(&mut self, by_num_pages: u32) -> Result<u32, ExecutorError> {
        (**self).memory_grow(by_num_pages)
    }

    fn memory_load_to_slice(&self, mem_ptr: MemPtr, dest: &mut [u8]) -> Result<(), ExecutorError> {
        (**self).memory_load_to_slice(mem_ptr, dest)
    }

    fn memory_load_owned(
        &self,
        mem_ptr: MemPtr,
        mem_length: MemLength,
    ) -> Result<Vec<u8>, ExecutorError> {
        (**self).memory_load_owned(mem_ptr, mem_length)
    }

    fn memory_store(&self, mem_ptr: MemPtr, data: &[u8]) -> Result<(), ExecutorError> {
        (**self).memory_store(mem_ptr, data)
    }

    /// Forwarded rather than left to the default, so that a state which overrides the
    /// clamping rules keeps its own behaviour when reached through a reference.
    fn memory_load_clamped(
        &self,
        mem_ptr: MemPtr,
        mem_length: MemLength,
    ) -> Result<Vec<u8>, ExecutorError> {
        (**self).memory_load_clamped(mem_ptr, mem_length)
    }

    /// Forwarded for the same reason: a backend that handles early exits must keep doing so
    /// when reached through a reference, rather than fall back to the failing default.
    fn handle_early_exit_legacy(
        &mut self,
        early_exit: VMHooksEarlyExit,
        breakpoint: BreakpointValueLegacy,
    ) -> Result<(), ExecutorError> {
        (**self).handle_early_exit_legacy(early_exit, breakpoint)
    }
}
