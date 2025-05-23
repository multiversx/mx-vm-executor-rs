use crate::middlewares::{get_points_used, set_points_used};
use crate::{ExperimentalError, ExperimentalInstanceInner};
use multiversx_chain_vm_executor::{ExecutorError, InstanceState};
use multiversx_chain_vm_executor::{MemLength, MemPtr};

use std::rc::Rc;
use std::rc::Weak;
use wasmer::{MemoryView, StoreMut};

pub struct ExperimentalInstanceState<'s> {
    pub wasmer_inner: Weak<ExperimentalInstanceInner>,
    pub store_mut: &'s mut StoreMut<'s>,
}

impl ExperimentalInstanceState<'_> {
    fn get_memory_view(&self) -> MemoryView<'_> {
        let wasmer_inner = self.wasmer_inner.upgrade().unwrap();
        wasmer_inner.get_memory_ref().unwrap().view(&self.store_mut)
    }

    fn get_wasmer_inner(&self) -> Result<Rc<ExperimentalInstanceInner>, ExecutorError> {
        self.wasmer_inner
            .upgrade()
            .ok_or_else(|| ExperimentalError::BadInstanceInnerPointer.into())
    }
}

impl InstanceState for &'_ mut ExperimentalInstanceState<'_> {
    fn get_points_used(&mut self) -> Result<u64, ExecutorError> {
        let wasmer_inner = self.get_wasmer_inner()?;
        let points_used = get_points_used(&wasmer_inner.wasmer_instance, &mut self.store_mut)?;
        Ok(points_used)
    }

    fn set_points_used(&mut self, points: u64) -> Result<(), ExecutorError> {
        let wasmer_inner = self.get_wasmer_inner()?;
        set_points_used(&wasmer_inner.wasmer_instance, &mut self.store_mut, points)?;
        Ok(())
    }

    fn memory_load_to_slice(&self, mem_ptr: MemPtr, dest: &mut [u8]) -> Result<(), ExecutorError> {
        let memory_view = self.get_memory_view();
        memory_view.read(mem_ptr as u64, dest)?;
        Ok(())
    }

    /// Copies data to new owned buffer.
    fn memory_load_owned(
        &self,
        mem_ptr: MemPtr,
        mem_length: MemLength,
    ) -> Result<Vec<u8>, ExecutorError> {
        let memory_view = self.get_memory_view();
        let len = mem_length as usize;
        let mut result = Vec::with_capacity(len);
        memory_view.read_uninit(mem_ptr as u64, result.spare_capacity_mut())?;
        unsafe {
            result.set_len(len);
        }
        Ok(result)
    }

    fn memory_store(&self, offset: MemPtr, data: &[u8]) -> Result<(), ExecutorError> {
        let memory_view = self.get_memory_view();
        memory_view.write(offset as u64, data)?;
        Ok(())
    }
}
