#![allow(unused)]

use crate::ExperimentalInstanceInner;
use crate::{we_imports::generate_import_object, we_vm_hooks::VMHooksWrapper};
use log::trace;
use multiversx_chain_vm_executor::{
    BreakpointValue, CompilationOptions, ExecutorError, InstanceFull, InstanceState, ServiceError,
};
use multiversx_chain_vm_executor::{MemLength, MemPtr};

use std::cell::RefCell;
use std::ops::{Add, Deref};
use std::rc::Weak;
use std::{rc::Rc, sync::Arc};
use wasmer::{
    imports, AsStoreMut, CompilerConfig, Extern, MemoryView, Module, Pages, Singlepass, Store,
    StoreMut,
};

const MAX_MEMORY_PAGES_ALLOWED: Pages = Pages(20);

pub struct ExperimentalInstanceState<'s> {
    pub wasmer_inner: Weak<ExperimentalInstanceInner>,
    pub store_mut: &'s mut StoreMut<'s>,
    pub breakpoint: BreakpointValue,
    pub points_limit: u64,
    pub points_used: u64,
}

impl ExperimentalInstanceState<'_> {
    fn get_memory_view(&self) -> MemoryView<'_> {
        let wasmer_inner = self.wasmer_inner.upgrade().unwrap();
        wasmer_inner.get_memory_ref().unwrap().view(&self.store_mut)
    }
}

impl InstanceState for &'_ mut ExperimentalInstanceState<'_> {
    fn get_points_limit(&self) -> Result<u64, String> {
        Ok(self.points_limit)
    }

    fn set_points_used(&mut self, points: u64) -> Result<(), String> {
        self.points_used = points;
        Ok(())
    }

    fn get_points_used(&self) -> Result<u64, String> {
        Ok(self.points_used)
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
        memory_view.read_uninit(mem_ptr as u64, result.spare_capacity_mut());
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

    fn set_breakpoint_value(&mut self, value: BreakpointValue) -> Result<(), String> {
        self.breakpoint = value;
        Ok(())
    }
}
