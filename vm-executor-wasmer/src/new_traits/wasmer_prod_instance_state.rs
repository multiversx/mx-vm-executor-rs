use crate::WasmerInstance;
use crate::executor_interface::{
    BreakpointValue, ExecutorError, InstanceLegacy, InstanceState, MemLength, MemPtr,
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

    pub fn set_breakpoint_value_legacy(&self, value: BreakpointValue) {
        self.instance_rc()
            .unwrap()
            .set_breakpoint_value(value.to_legacy())
            .expect("set_breakpoint_value_legacy globals error");
    }

    pub fn set_early_exit(&self, early_exit: VMHooksEarlyExit) {
        self.instance_rc().unwrap().set_early_exit(early_exit);
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
}
