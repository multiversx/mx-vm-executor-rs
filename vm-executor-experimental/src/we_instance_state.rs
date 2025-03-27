#![allow(unused)]

// use crate::wasmer_opcode_trace::OpcodeTracer;
use crate::we_protected_globals::ProtectedGlobals;
use crate::WasmerInstanceInner;
use crate::{
    we_breakpoints::*,
    we_imports::generate_import_object,
    // wasmer_metering::*,
    we_opcode_control::OpcodeControl,
    we_vm_hooks::VMHooksWrapper,
    WasmerExecutorData,
};
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

pub struct WasmerInstanceState {
    pub wasmer_inner: Weak<WasmerInstanceInner>,
    pub breakpoint: RefCell<BreakpointValue>,
    pub memory_ptr: *mut u8,
    pub memory_size: u64,
}

impl InstanceState for WasmerInstanceState {
    fn set_points_limit(&self, limit: u64) -> Result<(), String> {
        // set_points_limit(&self.wasmer_instance, limit)
        Ok(())
    }

    fn set_points_used(&self, points: u64) -> Result<(), String> {
        // set_points_used(&self.wasmer_instance, points)
        Ok(())
    }

    fn get_points_used(&self) -> Result<u64, String> {
        // get_points_used(&self.wasmer_instance)
        Ok(0)
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

    fn set_breakpoint_value(&self, value: BreakpointValue) -> Result<(), String> {
        *self.breakpoint.borrow_mut() = value;
        Ok(())
    }

    // fn get_breakpoint_value(&self) -> Result<BreakpointValue, String> {
    //     Ok((*self.breakpoint.borrow()))
    // }
}
