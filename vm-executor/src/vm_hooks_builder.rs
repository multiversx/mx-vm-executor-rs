use std::rc::Rc;

use crate::{InstanceState, VMHooks, VMHooksDefault};

pub trait VMHooksBuilder {
    fn create_vm_hooks(&self, instance_state_ref: Rc<dyn InstanceState>) -> Box<dyn VMHooks>;
}

#[derive(Debug)]
pub struct VMHooksBuilderDefault;

impl VMHooksBuilder for VMHooksBuilderDefault {
    fn create_vm_hooks(&self, _instance_state_ref: Rc<dyn InstanceState>) -> Box<dyn VMHooks> {
        Box::new(VMHooksDefault)
    }
}
