use crate::{InstanceState, VMHooks};

pub trait VMHooksBuilder {
    fn create_vm_hooks<'a, 'b, 'c>(&'a self, instance_state_ref: Box<dyn InstanceState + 'b>) -> Box<dyn VMHooks + 'c>;
}
