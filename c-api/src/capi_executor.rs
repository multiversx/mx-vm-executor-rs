//! Instantiate a module, call functions, and read exports.

use crate::{
    capi_vm_hook_pointers::vm_exec_vm_hook_c_func_pointers, capi_vm_hooks::CapiVMHooks,
    service_singleton::with_service, vm_exec_result_t,
};
use libc::c_void;
use multiversx_chain_vm_executor::Executor;
use multiversx_chain_vm_executor_wasmer::force_sighandler_reinstall;

#[repr(C)]
pub struct vm_exec_executor_t;

pub struct CapiExecutor {
    pub content: Box<dyn Executor>,
}

/// Creates a new VM executor.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_new_executor(
    executor: *mut *mut vm_exec_executor_t,
    vm_hook_pointers_ptr_ptr: *mut *mut vm_exec_vm_hook_c_func_pointers,
) -> vm_exec_result_t {
    return_if_ptr_null!(vm_hook_pointers_ptr_ptr, "VM hooks ptr is null");

    // unpacking the vm hooks object pointer
    let vm_hook_pointers_ptr = *vm_hook_pointers_ptr_ptr;
    let vm_hook_pointers = (*vm_hook_pointers_ptr).clone();

    // create executor
    let executor_result =
        with_service(|service| service.new_executor(Box::new(CapiVMHooks::new(vm_hook_pointers))));
    match executor_result {
        Ok(executor_box) => {
            let capi_executor = CapiExecutor {
                content: executor_box,
            };
            *executor = Box::into_raw(Box::new(capi_executor)) as *mut vm_exec_executor_t;
            vm_exec_result_t::VM_EXEC_OK
        }
        Err(message) => {
            with_service(|service| service.update_last_error_str(message.to_string()));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

/// Forces reinstalling the sighandlers.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[no_mangle]
pub unsafe extern "C" fn vm_force_sighandler_reinstall() {
    force_sighandler_reinstall();
}

/// Sets the data that can be hold by an instance context.
///
/// An instance context (represented by the opaque
/// `wasmer_instance_context_t` structure) can hold user-defined
/// data. This function sets the data. This function is complementary
/// of `wasmer_instance_context_data_get()`.
///
/// This function does nothing if `instance` is a null pointer.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_executor_set_vm_hooks_ptr(
    executor_ptr: *mut vm_exec_executor_t,
    vm_hooks_ptr: *mut c_void,
) -> vm_exec_result_t {
    let capi_executor = cast_input_ptr!(executor_ptr, CapiExecutor, "executor ptr is null");

    let result = capi_executor.content.set_vm_hooks_ptr(vm_hooks_ptr);
    match result {
        Ok(()) => vm_exec_result_t::VM_EXEC_OK,
        Err(message) => {
            with_service(|service| service.update_last_error_str(message.to_string()));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

/// Destroys a VM executor object.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_executor_destroy(executor_ptr: *mut vm_exec_executor_t) {
    if !executor_ptr.is_null() {
        let executor = Box::from_raw(executor_ptr as *mut CapiExecutor);
        drop(executor)
    }
}
