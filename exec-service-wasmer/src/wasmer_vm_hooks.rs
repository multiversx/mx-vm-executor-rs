use std::rc::Rc;

use elrond_exec_service::VMHooks;
use wasmer::WasmerEnv;

#[derive(Clone, Debug)]
pub struct VMHooksWrapper {
    pub vm_hooks: Rc<Box<dyn VMHooks>>,
}

unsafe impl Send for VMHooksWrapper {}
unsafe impl Sync for VMHooksWrapper {}

impl WasmerEnv for VMHooksWrapper {}
