//! Instantiate a module, call functions, and read exports.

use crate::{
    capi_vm_hook_pointers::vm_exec_vm_hook_pointers, capi_vm_hooks::CapiVmHooks,
    service_singleton::with_service, string_copy, string_length, vm_exec_byte_array,
    vm_exec_byte_array_list, vm_exec_result_t,
};
use elrond_exec_service::{CompilationOptions, Executor, ServiceInstance};
use libc::{c_char, c_int, c_void};
use std::{ffi::CStr, ptr, slice};

#[repr(C)]
pub struct vm_exec_executor_t;

pub struct CapiExecutor {
    pub content: Box<dyn Executor>,
}

#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_new_executor(
    executor: *mut *mut vm_exec_executor_t,
    vm_hook_pointers_ptr_raw: *mut c_void,
) -> vm_exec_result_t {
    if vm_hook_pointers_ptr_raw.is_null() {
        with_service(|service| service.update_last_error_str("VM hooks ptr is null".to_string()));
        return vm_exec_result_t::VM_EXEC_ERROR;
    }

    // unpacking the vm hooks object pointer
    let vm_hook_pointers_ptr_cast = vm_hook_pointers_ptr_raw as *mut *mut vm_exec_vm_hook_pointers;
    let vm_hook_pointers_ptr = *vm_hook_pointers_ptr_cast;
    let vm_hook_pointers = (*vm_hook_pointers_ptr).clone();

    // create executor
    let executor_result =
        with_service(|service| service.new_executor(Box::new(CapiVmHooks::new(vm_hook_pointers))));
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

/// Sets the data that can be hold by an instance context.
///
/// An instance context (represented by the opaque
/// `wasmer_instance_context_t` structure) can hold user-defined
/// data. This function sets the data. This function is complementary
/// of `wasmer_instance_context_data_get()`.
///
/// This function does nothing if `instance` is a null pointer.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_executor_set_context_ptr(
    executor: *mut vm_exec_executor_t,
    context_ptr: *mut c_void,
) -> vm_exec_result_t {
    // unpack the executor object
    if executor.is_null() {
        with_service(|service| service.update_last_error_str("executor ptr is null".to_string()));
        return vm_exec_result_t::VM_EXEC_ERROR;
    }
    let capi_executor = &mut *(executor as *mut CapiExecutor);

    let result = capi_executor.content.set_context_ptr(context_ptr);
    match result {
        Ok(()) => vm_exec_result_t::VM_EXEC_OK,
        Err(message) => {
            with_service(|service| service.update_last_error_str(message.to_string()));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn vm_exec_executor_destroy(executor: *mut vm_exec_executor_t) {
    if !executor.is_null() {
        unsafe {
            std::ptr::drop_in_place(executor);
        }
    }
}
