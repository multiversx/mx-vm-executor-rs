//! Instantiate a module, call functions, and read exports.

use crate::{
    service_singleton::with_service,
    string_copy,
    string_length,
    vm_exec_byte_array,
    vm_exec_byte_array_list,
    vm_exec_result_t,
};
use elrond_exec_service::{CompilationOptions, ServiceInstance, Executor};
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
) -> vm_exec_result_t {
    let executor_result = with_service(|service| service.new_executor());
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

#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub extern "C" fn vm_exec_executor_destroy(executor: *mut vm_exec_executor_t) {
    if !executor.is_null() {
        unsafe {
            std::ptr::drop_in_place(executor);
        }
    }
}
