use std::{rc::Weak, sync::Arc};

use multiversx_chain_vm_executor::{
    ExecutorRuntime, MemLength, MemPtr, VMHooks, VMHooksEarlyExit, with_vm_hooks_once,
};
use wasmer::FunctionEnvMut;

use crate::{ExperimentalInstanceInner, ExperimentalInstanceState};

pub struct VMHooksWrapper {
    pub runtime: Arc<dyn ExecutorRuntime>,
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

    with_vm_hooks_once(
        |lend| data.runtime.with_vm_hooks(&mut instance_state, lend),
        f,
    )
}
