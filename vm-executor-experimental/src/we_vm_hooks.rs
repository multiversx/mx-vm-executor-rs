use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use multiversx_chain_vm_executor::{BreakpointValue, MemLength, MemPtr, VMHooks, VMHooksBuilder};
use wasmer::FunctionEnvMut;

use crate::{
    middlewares::{get_points_limit, get_points_used, set_breakpoint_value, set_points_used},
    ExperimentalInstanceInner, ExperimentalInstanceState,
};

pub struct VMHooksWrapper {
    pub vm_hooks_builder: Rc<dyn VMHooksBuilder>,
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

pub fn with_vm_hooks<F, R>(mut env: FunctionEnvMut<VMHooksWrapper>, f: F) -> R
where
    F: FnOnce(&dyn VMHooks) -> R,
    R: Default + 'static,
{
    let (data, mut store_mut) = env.data_and_store_mut();

    let wasmer_inner = data.wasmer_inner.upgrade().unwrap();

    let points_limit = get_points_limit(&wasmer_inner.wasmer_instance, &mut store_mut).unwrap();
    let points_used = get_points_used(&wasmer_inner.wasmer_instance, &mut store_mut).unwrap();
    let memory_view = wasmer_inner.get_memory_ref().unwrap().view(&store_mut);

    let instance_state = Rc::new(ExperimentalInstanceState {
        wasmer_inner: data.wasmer_inner.clone(),
        breakpoint: RefCell::new(BreakpointValue::None),
        memory_ptr: memory_view.data_ptr(),
        memory_size: memory_view.data_size(),
        points_limit,
        points_used: RefCell::new(points_used),
    });

    let vm_hooks = data
        .vm_hooks_builder
        .create_vm_hooks(instance_state.clone());
    let result = f(&*vm_hooks);

    set_points_used(
        &wasmer_inner.wasmer_instance,
        &mut store_mut,
        *instance_state.points_used.borrow(),
    )
    .unwrap();

    let breakpoint = *instance_state.breakpoint.borrow();
    if breakpoint != BreakpointValue::None {
        set_breakpoint_value(
            &wasmer_inner.wasmer_instance,
            &mut store_mut,
            breakpoint.as_u64(),
        )
        .unwrap();
    }

    result
}
