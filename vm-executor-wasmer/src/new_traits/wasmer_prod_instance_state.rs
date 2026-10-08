use crate::WasmerInstance;
use crate::executor_interface::{
    BreakpointValueLegacy, ExecutorError, InstanceLegacy, InstanceState, MemLength, MemPtr,
    VMHooksEarlyExit,
};

use std::rc::{Rc, Weak};

use super::WasmerExecutorError;

#[derive(Clone)]
pub struct WasmerProdInstanceState {
    inner_instance_ref: Weak<WasmerInstance>,
}

impl WasmerProdInstanceState {
    pub fn new(inner_instance_ref: Weak<WasmerInstance>) -> Self {
        WasmerProdInstanceState { inner_instance_ref }
    }

    fn instance_rc(&self) -> Result<Rc<WasmerInstance>, ExecutorError> {
        self.inner_instance_ref
            .upgrade()
            .map_or_else(|| Err(WasmerExecutorError::BadInstancePointer.into()), Ok)
    }

}

impl InstanceState for WasmerProdInstanceState {
    fn get_points_used(&mut self) -> Result<u64, ExecutorError> {
        self.instance_rc()?
            .get_points_used()
            .map_err(|err| WasmerExecutorError::GetPointsUsed(err).into())
    }

    fn set_points_used(&mut self, points: u64) -> Result<(), ExecutorError> {
        self.instance_rc()?
            .set_points_used(points)
            .map_err(|err| WasmerExecutorError::SetPointsUsed(err).into())
    }

    fn memory_length(&self) -> Result<u64, ExecutorError> {
        self.instance_rc()?
            .memory_length()
            .map_err(|err| WasmerExecutorError::WrappedInstance(err).into())
    }

    fn memory_grow(&mut self, by_num_pages: u32) -> Result<u32, ExecutorError> {
        self.instance_rc()?.memory_grow(by_num_pages)
    }

    fn memory_load_to_slice(&self, mem_ptr: MemPtr, dest: &mut [u8]) -> Result<(), ExecutorError> {
        let instance_rc = self.instance_rc()?;
        let slice = instance_rc.memory_load(mem_ptr, dest.len() as MemLength)?;
        dest.copy_from_slice(slice);
        Ok(())
    }

    fn memory_load_owned(
        &self,
        mem_ptr: MemPtr,
        mem_length: MemLength,
    ) -> Result<Vec<u8>, ExecutorError> {
        let instance_rc = self.instance_rc()?;
        let slice = instance_rc.memory_load(mem_ptr, mem_length)?;
        Ok(slice.to_vec())
    }

    fn memory_store(&self, mem_ptr: MemPtr, data: &[u8]) -> Result<(), ExecutorError> {
        self.instance_rc()?.memory_store(mem_ptr, data)
    }

    /// The only backend that implements this: 2.2 is where legacy hooks run.
    fn handle_early_exit_legacy(
        &mut self,
        early_exit: VMHooksEarlyExit,
        breakpoint: BreakpointValueLegacy,
    ) -> Result<(), ExecutorError> {
        let instance_rc = self.instance_rc()?;

        // Trip the breakpoint global first: under 2.2 that is the mechanical halt, and it is
        // what stops the contract after the current instruction.
        instance_rc
            .set_breakpoint_value(breakpoint)
            .map_err(WasmerExecutorError::WrappedInstance)?;

        // Then stash the exit, for the error handler to pick up once the call unwinds.
        instance_rc.set_early_exit(early_exit);

        Ok(())
    }
}
