use std::{
    borrow::Borrow,
    ffi::c_void,
    sync::{Arc, Mutex}, ops::Deref, rc::Rc,
};

use elrond_exec_service::VMHooks;
use wasmer::WasmerEnv;

#[derive(Clone)]
pub struct VMHooksWrapper {
    pub vm_hooks: Rc<Box<dyn VMHooks>>,
}

impl VMHooksWrapper {
    pub fn new<H: VMHooks>(vm_hooks: H) -> Self {
        Self {
            vm_hooks: Rc::new(Box::new(vm_hooks)),
        }
    }

    // pub fn with_vm_hooks<F, R>(&self, f: F) -> R
    // where
    //     F: FnOnce(&Box<dyn VMHooks>) -> R,
    // {
    //     let vm_hooks_instance = self.vm_hooks.deref();
    //     f(&*vm_hooks_instance)
    // }

    // pub fn set_context_ptr(&self, context_ptr: *mut c_void) {
    //     let mut content = self.0.lock().unwrap();
    //     content.context_ptr = context_ptr;
    // }
}

unsafe impl Send for VMHooksWrapper {}
unsafe impl Sync for VMHooksWrapper {}

impl WasmerEnv for VMHooksWrapper {}
