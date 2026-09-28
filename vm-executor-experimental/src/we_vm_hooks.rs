use std::rc::Weak;

use multiversx_chain_vm_executor::{MemLength, MemPtr, VMHooks, VMHooksEarlyExit};
use wasmer::FunctionEnvMut;

use crate::{ExperimentalInstanceInner, ExperimentalInstanceState, ExperimentalVMHooksBuilder};

pub struct VMHooksWrapper {
    pub vm_hooks_builder: Box<dyn ExperimentalVMHooksBuilder>,
    pub wasmer_inner: Weak<ExperimentalInstanceInner>,
}

unsafe impl Send for VMHooksWrapper {}
unsafe impl Sync for VMHooksWrapper {}

pub fn convert_mem_ptr(raw: i32) -> MemPtr {
    raw as MemPtr
}

pub fn convert_mem_length(raw: i32) -> MemLength {
    raw as MemLength
}

pub fn with_vm_hooks<F, R>(
    mut env: FunctionEnvMut<VMHooksWrapper>,
    f: F,
) -> Result<R, VMHooksEarlyExit>
where
    F: FnOnce(&mut dyn VMHooks) -> Result<R, VMHooksEarlyExit>,
    R: Default + 'static,
{
    let (data, mut store_mut) = env.data_and_store_mut();

    let mut instance_state = ExperimentalInstanceState {
        wasmer_inner: data.wasmer_inner.clone(),
        store_mut: &mut store_mut,
    };

    // The builder lends the hooks through an `FnMut`, while `f` is an `FnOnce`,
    // so it gets moved out on the (single) call.
    let mut f = Some(f);
    let mut result = None;

    data.vm_hooks_builder
        .with_vm_hooks(&mut instance_state, &mut |vm_hooks| {
            let f = f.take().expect("VM hooks builder called the closure twice");
            result = Some(f(vm_hooks));
        });

    result.expect("VM hooks builder did not call the closure")
}
