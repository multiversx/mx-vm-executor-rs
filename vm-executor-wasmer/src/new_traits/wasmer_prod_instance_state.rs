use crate::WasmerInstance;
use anyhow::anyhow;
use multiversx_chain_vm_executor::{
    BreakpointValue, ExecutorError, InstanceLegacy, InstanceState, MemLength, MemPtr,
    VMHooksEarlyExit,
};

use std::rc::{Rc, Weak};

#[derive(Clone)]
pub struct WasmerProdInstanceState {
    inner_instance_ref: Weak<WasmerInstance>,
}

impl WasmerProdInstanceState {
    pub fn new(inner_instance_ref: Weak<WasmerInstance>) -> Self {
        WasmerProdInstanceState { inner_instance_ref }
    }

    fn instance_rc(&self) -> anyhow::Result<Rc<WasmerInstance>> {
        self.inner_instance_ref
            .upgrade()
            .map_or_else(|| Err(anyhow!("bad wasmer instance pointer")), Ok)
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
    // TODO: delete
    fn get_points_limit(&self) -> Result<u64, ExecutorError> {
        Err(anyhow!("get_points_limit not supported, to be deleted").into())
    }

    fn set_points_used(&mut self, points: u64) -> Result<(), ExecutorError> {
        self.instance_rc()?
            .set_points_used(points)
            .map_err(|err| anyhow!("globals error: {err}").into())
    }

    fn get_points_used(&self) -> Result<u64, ExecutorError> {
        self.instance_rc()?
            .get_points_used()
            .map_err(|err| anyhow!("globals error: {err}").into())
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

    fn set_breakpoint_value(&mut self, value: BreakpointValue) -> Result<(), ExecutorError> {
        self.instance_rc()?
            .set_breakpoint_value(value.to_legacy())
            .map_err(|err| anyhow!("globals error: {err}").into())
    }
}
