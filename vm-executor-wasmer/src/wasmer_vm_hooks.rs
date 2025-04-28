use std::{cell::RefCell, rc::Rc};

use multiversx_chain_vm_executor::{MemLength, MemPtr, VMHooksLegacy};
use wasmer::WasmerEnv;

#[derive(Clone, Debug)]
pub struct VMHooksWrapper {
    pub vm_hooks: Rc<RefCell<Box<dyn VMHooksLegacy>>>,
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

    pub fn with_vm_hooks<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut dyn VMHooksLegacy) -> R,
    {
        let mut vm_hooks = self.vm_hooks.borrow_mut();
        f(&mut **vm_hooks)
    }
}
