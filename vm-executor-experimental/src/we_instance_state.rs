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
    imports, AsStoreMut, CompilerConfig, Extern, Module, Pages, Singlepass, Store, StoreMut,
};

const MAX_MEMORY_PAGES_ALLOWED: Pages = Pages(20);

pub struct ExperimentalInstanceState {
    pub wasmer_inner: Weak<ExperimentalInstanceInner>,
    pub breakpoint: RefCell<BreakpointValue>,
    pub memory_ptr: *mut u8,
    pub memory_size: u64,
    pub points_limit: u64,
    pub points_used: RefCell<u64>,
}

impl InstanceState for ExperimentalInstanceState {
    fn get_points_limit(&self) -> Result<u64, String> {
        Ok(self.points_limit)
    }

    fn set_points_used(&mut self, points: u64) -> Result<(), String> {
        *self.points_used.borrow_mut() = points;
        Ok(())
    }

    fn get_points_used(&self) -> Result<u64, String> {
        Ok(*self.points_used.borrow())
    }

    fn memory_length(&self) -> Result<u64, String> {
        Ok(self.memory_size)
    }

    fn memory_ptr(&self) -> Result<*mut u8, String> {
        Ok(self.memory_ptr)
    }

    fn memory_load(&self, offset: MemPtr, mem_length: MemLength) -> Result<&[u8], ExecutorError> {
        let memory_ptr = self.memory_ptr()?;
        let ptr = unsafe { memory_ptr.offset(offset) };
        let slice = std::ptr::slice_from_raw_parts(ptr, mem_length as usize);
        unsafe { Ok(&*slice) }
    }

    fn memory_store(&self, offset: MemPtr, data: &[u8]) -> Result<(), ExecutorError> {
        let memory_ptr = self.memory_ptr()?;
        let ptr = unsafe { memory_ptr.offset(offset) };
        let slice = std::ptr::slice_from_raw_parts(ptr, data.len());
        unsafe {
            std::ptr::copy(data.as_ptr(), ptr, data.len());
        }
        Ok(())
    }

    fn memory_grow(&self, by_num_pages: u32) -> Result<u32, ExecutorError> {
        todo!()
    }

    fn set_breakpoint_value(&mut self, value: BreakpointValue) -> Result<(), String> {
        *self.breakpoint.borrow_mut() = value;
        Ok(())
    }
}
