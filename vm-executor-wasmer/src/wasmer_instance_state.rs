use crate::{wasmer_breakpoints::*, wasmer_metering::*};
use anyhow::anyhow;
use multiversx_chain_vm_executor::{BreakpointValue, ExecutorError, InstanceState};
use multiversx_chain_vm_executor::{MemLength, MemPtr};

// TODO: delete
pub struct WasmerInstanceState<'a> {
    pub(crate) wasmer_instance: &'a wasmer::Instance,
    pub(crate) memory_name: &'a str,
}

impl WasmerInstanceState<'_> {
    fn get_memory_ref(&self) -> Result<&wasmer::Memory, String> {
        let result = self.wasmer_instance.exports.get_memory(self.memory_name);
        match result {
            Ok(memory) => Ok(memory),
            Err(err) => Err(err.to_string()),
        }
    }

    fn memory_slice(&self, mem_ptr: MemPtr, mem_length: usize) -> Result<&[u8], ExecutorError> {
        let result = self.get_memory_ref();
        match result {
            Ok(memory) => unsafe {
                let mem_data = memory.data_unchecked();
                let start = mem_ptr as usize;
                let end = mem_ptr as usize + mem_length;
                Ok(&mem_data[start..end])
            },
            Err(err) => Err(err.into()),
        }
    }
}

impl InstanceState for WasmerInstanceState<'_> {
    fn get_points_limit(&self) -> Result<u64, ExecutorError> {
        get_points_limit(self.wasmer_instance).map_err(|err| anyhow!("globals error: {err}").into())
    }

    fn set_points_used(&mut self, points: u64) -> Result<(), ExecutorError> {
        set_points_used(self.wasmer_instance, points)
            .map_err(|err| anyhow!("globals error: {err}").into())
    }

    fn get_points_used(&self) -> Result<u64, ExecutorError> {
        get_points_used(self.wasmer_instance).map_err(|err| anyhow!("globals error: {err}").into())
    }

    fn memory_load_to_slice(&self, mem_ptr: MemPtr, dest: &mut [u8]) -> Result<(), ExecutorError> {
        let slice = self.memory_slice(mem_ptr, dest.len())?;
        dest.copy_from_slice(slice);
        Ok(())
    }

    fn memory_load_owned(
        &self,
        mem_ptr: MemPtr,
        mem_length: MemLength,
    ) -> Result<Vec<u8>, ExecutorError> {
        let slice = self.memory_slice(mem_ptr, mem_length as usize)?;
        Ok(slice.to_vec())
    }

    fn memory_store(&self, mem_ptr: MemPtr, data: &[u8]) -> Result<(), ExecutorError> {
        let result = self.get_memory_ref();
        match result {
            Ok(memory) => unsafe {
                let mem_data = memory.data_unchecked_mut();
                mem_data[mem_ptr as usize..mem_ptr as usize + data.len()].copy_from_slice(data);
                Ok(())
            },
            Err(err) => Err(err.into()),
        }
    }

    fn set_breakpoint_value(&mut self, value: BreakpointValue) -> Result<(), ExecutorError> {
        set_breakpoint_value(self.wasmer_instance, value.as_u64())
            .map_err(|err| anyhow!("globals error: {err}").into())
    }
}
