use crate::{InstanceState, VMHooks, VMHooksDefault};

pub trait VMHooksBuilder {
    fn create_vm_hooks<'b, 'h>(
        &'b self,
        instance_state_ref: &'h mut dyn InstanceState,
    ) -> Box<dyn VMHooks + 'h>;
}

#[derive(Debug)]
pub struct VMHooksBuilderDefault;

impl VMHooksBuilder for VMHooksBuilderDefault {
    fn create_vm_hooks<'b, 'h>(
        &'b self,
        _instance_state_ref: &'h mut dyn InstanceState,
    ) -> Box<dyn VMHooks + 'h> {
        Box::new(VMHooksDefault)
    }
}

impl VMHooksBuilderDefault {
    pub fn wrap<'h>(
        &self,
        _instance_state_ref: &'h mut dyn InstanceState,
    ) -> Box<dyn VMHooks + 'h> {
        Box::new(VMHooksDefault)
    }
}
