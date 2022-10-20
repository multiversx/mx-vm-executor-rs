use std::{
    borrow::Borrow,
    ffi::c_void,
    sync::{Arc, Mutex}, ops::Deref, rc::Rc,
};

use elrond_exec_service::VMHooks;
use wasmer::WasmerEnv;

#[derive(Clone, Debug)]
pub struct VMHooksWrapper {
    pub vm_hooks: Rc<Box<dyn VMHooks>>,
}

// impl VMHooksWrapper {
//     pub fn new<H: VMHooks>(vm_hooks: H) -> Self {
//         Self {
//             vm_hooks: Rc::new(Box::new(vm_hooks)),
//         }
//     }
// }

unsafe impl Send for VMHooksWrapper {}
unsafe impl Sync for VMHooksWrapper {}

impl WasmerEnv for VMHooksWrapper {}
