use std::rc::Rc;

use crate::{InstanceState, VMHooks};

pub trait VMHooksBuilder {
    // fn create_vm_hooks<'a, 'b, 'c>(&'a self, instance_state_ref: &'b dyn InstanceState) -> Box<dyn VMHooks + 'c>;

    // fn create_vm_hooks(&self, instance_state_ref: Rc<dyn InstanceState>) -> Box<dyn VMHooks>;

    fn with_vm_hooks(&self, instance_state_ref: &dyn InstanceState, f: &dyn FnOnce(&dyn VMHooks));
}
