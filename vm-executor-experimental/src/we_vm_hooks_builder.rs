use multiversx_chain_vm_executor::{VMHooks, VMHooksDefault};

use crate::ExperimentalInstanceState;

pub trait ExperimentalVMHooksBuilder {
    fn create_vm_hooks<'b, 'h>(
        &'b self,
        instance_state_ref: &'h mut ExperimentalInstanceState,
    ) -> Box<dyn VMHooks + 'h>;
}

#[derive(Debug)]
pub struct ExperimentalVMHooksBuilderDefault;

impl ExperimentalVMHooksBuilder for ExperimentalVMHooksBuilderDefault {
    fn create_vm_hooks<'b, 'h>(
        &'b self,
        _instance_state_ref: &'h mut ExperimentalInstanceState,
    ) -> Box<dyn VMHooks + 'h> {
        Box::new(VMHooksDefault)
    }
}
