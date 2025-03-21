use crate::{wasmer_breakpoints::*, wasmer_metering::*};
use multiversx_chain_vm_executor::{BreakpointValue, ExecutorError, InstanceState};
use multiversx_chain_vm_executor::{MemLength, MemPtr};

pub struct WasmerInstanceState<'a> {
    pub(crate) wasmer_instance: &'a wasmer::Instance,
    pub(crate) memory_name: &'a str,
}

impl WasmerInstanceState<'_> {
    fn get_memory_ref(&self) -> Result<&wasmer::Memory, String> {
        let result = self.wasmer_instance.exports.get_memory(&self.memory_name);
        match result {
            Ok(memory) => Ok(memory),
            Err(err) => Err(err.to_string()),
        }
    }
}

impl InstanceState for WasmerInstanceState<'_> {
    fn set_points_limit(&self, limit: u64) -> Result<(), String> {
        set_points_limit(&self.wasmer_instance, limit)
    }

    fn set_points_used(&self, points: u64) -> Result<(), String> {
        set_points_used(&self.wasmer_instance, points)
    }

    fn get_points_used(&self) -> Result<u64, String> {
        get_points_used(&self.wasmer_instance)
    }

    fn memory_length(&self) -> Result<u64, String> {
        let result = self.get_memory_ref();
        match result {
            Ok(memory) => Ok(memory.data_size()),
            Err(err) => Err(err),
        }
    }

    fn memory_ptr(&self) -> Result<*mut u8, String> {
        let result = self.get_memory_ref();
        match result {
            Ok(memory) => Ok(memory.data_ptr()),
            Err(err) => Err(err),
        }
    }

    fn memory_load(&self, mem_ptr: MemPtr, mem_length: MemLength) -> Result<&[u8], ExecutorError> {
        let result = self.get_memory_ref();
        match result {
            Ok(memory) => unsafe {
                let mem_data = memory.data_unchecked();
                let start = mem_ptr as usize;
                let end = (mem_ptr + mem_length) as usize;
                Ok(&mem_data[start..end])
            },
            Err(err) => Err(err.into()),
        }
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

    fn memory_grow(&mut self, by_num_pages: u32) -> Result<u32, ExecutorError> {
        let result = self.get_memory_ref();
        match result {
            Ok(memory) => {
                let pages = memory.grow(wasmer::Pages(by_num_pages))?;
                Ok(pages.0)
            }
            Err(err) => Err(err.into()),
        }
    }

    fn set_breakpoint_value(&mut self, value: BreakpointValue) -> Result<(), String> {
        set_breakpoint_value(&self.wasmer_instance, value.as_u64())
    }

    fn get_breakpoint_value(&mut self) -> Result<BreakpointValue, String> {
        get_breakpoint_value(&self.wasmer_instance)?.try_into()
    }
}
