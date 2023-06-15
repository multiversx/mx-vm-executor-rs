use std::rc::Rc;

use multiversx_chain_vm_executor::{MemLength, MemPtr, VMHooks};
use wasmer::WasmerEnv;

#[derive(Clone, Debug)]
pub struct VMHooksWrapper {
    pub vm_hooks: Rc<Box<dyn VMHooks>>,
}

unsafe impl Send for VMHooksWrapper {}
unsafe impl Sync for VMHooksWrapper {}

impl WasmerEnv for VMHooksWrapper {}

impl VMHooksWrapper {
    pub(crate) fn convert_mem_ptr(&self, raw: i32) -> MemPtr {
        raw as MemPtr
    }

    pub(crate) fn convert_mem_length(&self, raw: i32) -> MemLength {
        raw as MemLength
    }
}
