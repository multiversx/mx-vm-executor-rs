use std::{
    cell::RefCell, mem::MaybeUninit, rc::{Rc, Weak}
};

use multiversx_chain_vm_executor::{BreakpointValue, MemLength, MemPtr, VMHooks, VMHooksBuilder};
use wasmer::FunctionEnvMut;

use crate::{wasmer_breakpoints::set_breakpoint_value, WasmerInstanceInner, WasmerInstanceState};

// #[derive(Clone, Debug)]
pub struct VMHooksWrapper {
    // pub vm_hooks: Rc<Box<dyn VMHooks>>,
    pub vm_hooks_builder: Box<dyn VMHooksBuilder>,
    pub wasmer_inner: Weak<WasmerInstanceInner>,
}

unsafe impl Send for VMHooksWrapper {}
unsafe impl Sync for VMHooksWrapper {}

// impl VMHooksWrapper {
//     pub(crate) fn convert_mem_ptr(&self, raw: i32) -> MemPtr {
//         raw as MemPtr
//     }

//     pub(crate) fn convert_mem_length(&self, raw: i32) -> MemLength {
//         raw as MemLength
//     }
// }

pub fn convert_mem_ptr(raw: i32) -> MemPtr {
    raw as MemPtr
}

pub fn convert_mem_length(raw: i32) -> MemLength {
    raw as MemLength
}


pub fn with_vm_hooks<F, R>(mut env: FunctionEnvMut<VMHooksWrapper>, f: F) -> R
where
    F: FnOnce(&dyn VMHooks) -> R,
    R: Default,
{
    let (data, mut store_mut) = env.data_and_store_mut();

    let wasmer_inner = data.wasmer_inner.upgrade().unwrap();

    let memory_view = wasmer_inner.get_memory_ref().unwrap().view(&store_mut);

    // let inner = dummy_wasmer_inner();
    let instance_state = WasmerInstanceState {
        // wasmer_inner: data.wasmer_inner.as_ptr().as_ref().unwrap(),
        wasmer_inner: data.wasmer_inner.clone(),
        memory_view,
        breakpoint: RefCell::new(BreakpointValue::None),
    };
    // let instance_state_2 = WasmerInstanceState2 { env };
    let mut result = R::default();
    data
        .vm_hooks_builder
        .with_vm_hooks(&instance_state, &|vm_hooks| {
            result = f(vm_hooks);
        });

    let breakpoint = *instance_state.breakpoint.borrow();
    if breakpoint != BreakpointValue::None {
        set_breakpoint_value(
            &data.wasmer_inner.upgrade().unwrap().wasmer_instance,
            &mut store_mut,
            breakpoint.as_u64(),
        ).unwrap();
    }

    result
}


