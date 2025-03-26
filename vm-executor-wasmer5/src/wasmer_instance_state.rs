#![allow(unused)]

// use crate::wasmer_opcode_trace::OpcodeTracer;
use crate::wasmer_protected_globals::ProtectedGlobals;
use crate::WasmerInstanceInner;
use crate::{
    wasmer5_imports::generate_import_object,
    wasmer_breakpoints::*,
    // wasmer_metering::*,
    wasmer_opcode_control::OpcodeControl,
    wasmer_vm_hooks::VMHooksWrapper,
    WasmerExecutorData,
};
use log::trace;
use multiversx_chain_vm_executor::{
    BreakpointValue, CompilationOptions, ExecutorError, Instance, InstanceState, ServiceError,
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
    // pub memory: &'a wasmer::Memory,
    // pub store_ref: StoreMut<'a>,
    // pub memory_view: wasmer::MemoryView<'a>,
    pub breakpoint: RefCell<BreakpointValue>,
    pub memory_ptr: *mut u8,
    pub memory_size: u64,
}

impl WasmerInstanceState {
    // fn get_memory_ref(&self) -> Result<&wasmer::Memory, String> {
    //     let inner = self.wasmer_inner .upgrade().unwrap();
    //     let result = inner
    //         .wasmer_instance
    //         .exports
    //         .get_memory(&inner.memory_name);
    //     match result {
    //         Ok(memory) => Ok(memory),
    //         Err(err) => Err(err.to_string()),
    //     }
    // }
}

// fn get_memories(wasmer_instance: &wasmer::Instance) -> Vec<(&String, &wasmer::Memory)> {
//     let memories = wasmer_instance
//         .exports
//         .iter()
//         .memories()
//         .collect::<Vec<_>>();
//     memories
// }

// fn validate_memories(memories: &[(&String, &wasmer::Memory)]) -> Result<(), ExecutorError> {
//     if memories.is_empty() {
//         return Err(Box::new(ServiceError::new(
//             "no memory declared in smart contract",
//         )));
//     }
//     if memories.len() > 1 {
//         return Err(Box::new(ServiceError::new(
//             "more than one memory declared in smart contract",
//         )));
//     }

//     Ok(())
// }

// fn validate_memory(memory: &wasmer::Memory, store: &wasmer::Store) -> Result<(), ExecutorError> {
//     let memory_type = memory.ty(store);
//     let max_memory_pages = memory_type.maximum.unwrap_or(memory_type.minimum);

//     if max_memory_pages > MAX_MEMORY_PAGES_ALLOWED {
//         trace!(
//             "Memory size exceeds maximum allowed: {:#?} > {:#?}",
//             max_memory_pages,
//             MAX_MEMORY_PAGES_ALLOWED
//         );
//         return Err(Box::new(ServiceError::new(
//             "memory size exceeds maximum allowed",
//         )));
//     }

//     Ok(())
// }

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
        // let result = self.get_memory_ref();
        // match result {
        //     Ok(memory) => Ok(memory.view(&self.store_ref).data_size()),
        //     Err(err) => Err(err),
        // }

        Ok(self.memory_size)
    }

    fn memory_ptr(&self) -> Result<*mut u8, String> {
        // let result = self.get_memory_ref();
        // match result {
        //     Ok(memory) => Ok(memory.view(&self.store_ref).data_ptr()),
        //     Err(err) => Err(err),
        // }

        Ok(self.memory_ptr)
    }

    fn memory_load(&self, offset: MemPtr, mem_length: MemLength) -> Result<&[u8], ExecutorError> {
        let memory_ptr = self.memory_ptr()?;
        let ptr = unsafe { memory_ptr.offset(offset) };
        let slice = std::ptr::slice_from_raw_parts(ptr, mem_length as usize);
        unsafe { Ok(&*slice) }

        // let result = self.get_memory_ref();
        // match result {
        //     Ok(memory) => unsafe {
        //         let view = memory.view(&self.wasmer_store);
        //         let mem_data = view.read(offset, buf) memory.data_unchecked();
        //         let start = mem_ptr as usize;
        //         let end = (mem_ptr + mem_length) as usize;
        //         Ok(&mem_data[start..end])
        //     },
        //     Err(err) => Err(err.into()),
        // }

        // let mem_data = self.memory_view.read(mem_ptr as u64, buf);
        // let start = mem_ptr as usize;
        // let end = (mem_ptr + mem_length) as usize;
        // Ok(&mem_data[start..end])
    }

    fn memory_store(&self, offset: MemPtr, data: &[u8]) -> Result<(), ExecutorError> {
        let memory_ptr = self.memory_ptr()?;
        let ptr = unsafe { memory_ptr.offset(offset) };
        let slice = std::ptr::slice_from_raw_parts(ptr, data.len());
        unsafe {
            std::ptr::copy(data.as_ptr(), ptr, data.len());
        }
        Ok(())

        // let result = self.get_memory_ref();
        // match result {
        //     Ok(memory) => unsafe {
        //         // let store_ref = self.wasmer_store.borrow();
        //         let view = memory.view(&self.store_ref);
        //         view.write(mem_ptr as u64, data);
        //         // let mem_data = memory.data_unchecked_mut();
        //         // mem_data[mem_ptr as usize..mem_ptr as usize + data.len()].copy_from_slice(data);
        //         Ok(())
        //     },
        //     Err(err) => Err(err.into()),
        // }
    }

    fn memory_grow(&self, by_num_pages: u32) -> Result<u32, ExecutorError> {
        // let result = self.get_memory_ref();
        // match result {
        //     Ok(memory) => {
        //         let pages = memory.grow(&mut self.wasmer_store, wasmer::Pages(by_num_pages))?;
        //         Ok(pages.0)
        //     }
        //     Err(err) => Err(err.into()),
        // }
        todo!()
    }

    fn set_breakpoint_value(&self, value: BreakpointValue) -> Result<(), String> {
        // set_breakpoint_value(
        //     &self.wasmer_inner.wasmer_instance,
        //     &mut self.store_ref,
        //     value.as_u64(),
        // )
        *self.breakpoint.borrow_mut() = value;
        Ok(())
    }

    fn get_breakpoint_value(&self) -> Result<BreakpointValue, String> {
        // get_breakpoint_value(&self.wasmer_inner.wasmer_instance, &mut self.store_ref)?.try_into()
        Ok((*self.breakpoint.borrow()))
    }
}
