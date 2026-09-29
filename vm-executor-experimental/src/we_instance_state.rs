use crate::middlewares::{get_points_used, set_points_used};
use crate::{ExperimentalError, ExperimentalInstanceInner};
use multiversx_chain_vm_executor::{ExecutorError, InstanceState};
use multiversx_chain_vm_executor::{MemLength, MemPtr, checked_load_range, checked_store_range};

use std::rc::Rc;
use std::rc::Weak;
use wasmer::{MemoryView, Pages, StoreMut};

pub struct ExperimentalInstanceState<'s> {
    pub wasmer_inner: Weak<ExperimentalInstanceInner>,
    pub store_mut: &'s mut StoreMut<'s>,
}

impl ExperimentalInstanceState<'_> {
    fn get_memory_view(&self) -> Result<MemoryView<'_>, ExecutorError> {
        let wasmer_inner = self.get_wasmer_inner()?;
        let memory = wasmer_inner
            .get_memory_ref()
            .map_err(ExperimentalError::MemoryAccess)?;

        // The view borrows the store, not `wasmer_inner`, which is why it outlives the `Rc`
        // obtained above. The memory itself is owned by the instance, kept alive by the
        // caller for the duration of the host call.
        Ok(memory.view(&self.store_mut))
    }

    fn memory_size(&self) -> Result<u64, ExecutorError> {
        Ok(self.get_memory_view()?.data_size())
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

    fn memory_length(&self) -> Result<u64, ExecutorError> {
        self.memory_size()
    }

    fn memory_grow(&mut self, by_num_pages: u32) -> Result<u32, ExecutorError> {
        let wasmer_inner = self.get_wasmer_inner()?;
        let memory = wasmer_inner
            .get_memory_ref()
            .map_err(ExperimentalError::MemoryAccess)?;
        let pages = memory.grow(&mut self.store_mut, Pages(by_num_pages))?;
        Ok(pages.0)
    }

    fn memory_load_to_slice(&self, mem_ptr: MemPtr, dest: &mut [u8]) -> Result<(), ExecutorError> {
        let memory_view = self.get_memory_view()?;
        let (start, _) =
            checked_load_range(mem_ptr, dest.len() as MemLength, memory_view.data_size())?;
        memory_view.read(start as u64, dest)?;
        Ok(())
    }

    /// Copies data to new owned buffer.
    fn memory_load_owned(
        &self,
        mem_ptr: MemPtr,
        mem_length: MemLength,
    ) -> Result<Vec<u8>, ExecutorError> {
        let memory_view = self.get_memory_view()?;
        let (start, end) = checked_load_range(mem_ptr, mem_length, memory_view.data_size())?;
        let len = end - start;
        let mut result = Vec::with_capacity(len);
        memory_view.read_uninit(start as u64, result.spare_capacity_mut())?;
        unsafe {
            result.set_len(len);
        }
        Ok(result)
    }

    fn memory_store(&self, offset: MemPtr, data: &[u8]) -> Result<(), ExecutorError> {
        let memory_view = self.get_memory_view()?;
        let (start, _) = checked_store_range(offset, data.len(), memory_view.data_size())?;
        memory_view.write(start as u64, data)?;
        Ok(())
    }
}
