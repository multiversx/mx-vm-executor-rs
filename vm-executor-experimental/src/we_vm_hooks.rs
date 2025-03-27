use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use multiversx_chain_vm_executor::{BreakpointValue, MemLength, MemPtr, VMHooks, VMHooksBuilder};
use wasmer::FunctionEnvMut;

use crate::{we_breakpoints::set_breakpoint_value, WasmerInstanceInner, WasmerInstanceState};

// #[derive(Clone, Debug)]
pub struct VMHooksWrapper {
    pub vm_hooks_builder: Rc<dyn VMHooksBuilder>,
    pub wasmer_inner: Weak<WasmerInstanceInner>,
}

unsafe impl Send for VMHooksWrapper {}
unsafe impl Sync for VMHooksWrapper {}

pub fn convert_mem_ptr(raw: i32) -> MemPtr {
    raw as MemPtr
}

pub fn convert_mem_length(raw: i32) -> MemLength {
    raw as MemLength
}

pub fn with_vm_hooks<F, R>(mut env: FunctionEnvMut<VMHooksWrapper>, f: F) -> R
where
    F: FnOnce(&dyn VMHooks) -> R,
    R: Default + 'static,
{
    let (data, mut store_mut) = env.data_and_store_mut();

    let wasmer_inner = data.wasmer_inner.upgrade().unwrap();

    let memory_view = wasmer_inner.get_memory_ref().unwrap().view(&store_mut);

    let instance_state = Rc::new(WasmerInstanceState {
        wasmer_inner: data.wasmer_inner.clone(),
        breakpoint: RefCell::new(BreakpointValue::None),
        memory_ptr: memory_view.data_ptr(),
        memory_size: memory_view.data_size(),
    });

    let vm_hooks = data
        .vm_hooks_builder
        .create_vm_hooks(instance_state.clone());
    let result = f(&*vm_hooks);

    let breakpoint = *instance_state.breakpoint.borrow();
    if breakpoint != BreakpointValue::None {
        set_breakpoint_value(
            &data.wasmer_inner.upgrade().unwrap().wasmer_instance,
            &mut store_mut,
            breakpoint.as_u64(),
        )
        .unwrap();
    }

    result
}
