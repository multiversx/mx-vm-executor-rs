//! Instantiate a module, call functions, and read exports.

use crate::{
    capi_instance::{vm_exec_instance_t, CapiInstance},
    service_singleton::with_service,
    vm_exec_result_t,
};
use std::ptr;

/// Gets the size in bytes of the memory data.
///
/// This function returns 0 if `memory` is a null pointer.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_memory_data_length(
    instance_ptr: *mut vm_exec_instance_t,
) -> u64 {
    let capi_instance = cast_input_ptr!(instance_ptr, CapiInstance, "instance ptr is null", 0);
    let result = capi_instance.content.memory_length();
    match result {
        Ok(length) => length,
        Err(err) => {
            with_service(|service| service.update_last_error_str(err));
            0
        }
    }
}

/// Gets a pointer to the beginning of the contiguous memory data
/// bytes.
///
/// The function returns `NULL` if `memory` is a null pointer.
///
/// Note that when the memory grows, it can be reallocated, and thus
/// the returned pointer can be invalidated.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_memory_data(
    instance_ptr: *mut vm_exec_instance_t,
) -> *mut u8 {
    let capi_instance = cast_input_ptr!(
        instance_ptr,
        CapiInstance,
        "instance ptr is null",
        ptr::null_mut()
    );
    let result = capi_instance.content.memory_ptr();
    match result {
        Ok(data) => data,
        Err(err) => {
            with_service(|service| service.update_last_error_str(err));
            ptr::null_mut()
        }
    }
}

/// Grows a memory by the given number of pages (of 65Kb each).
///
/// The functions return `wasmer_result_t::WASMER_OK` upon success,
/// `wasmer_result_t::WASMER_ERROR` otherwise. Use
/// `wasmer_last_error_length()` with `wasmer_last_error_message()` to
/// read the error message.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_memory_grow(
    instance_ptr: *mut vm_exec_instance_t,
    by_num_pages: u32,
) -> vm_exec_result_t {
    let capi_instance = cast_input_ptr!(instance_ptr, CapiInstance, "instance ptr is null");
    let grow_result = capi_instance.content.memory_grow(by_num_pages);

    match grow_result {
        Ok(_) => vm_exec_result_t::VM_EXEC_OK,
        Err(grow_error) => {
            with_service(|service| service.update_last_error_str(grow_error.to_string()));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}
