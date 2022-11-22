use crate::capi_instance::{vm_exec_instance_t, CapiInstance};
use crate::service_singleton::with_service;
use crate::vm_exec_result_t;

/// Gets the size in bytes of the memory data.
///
/// This function returns `vm_exec_result_t::WASMER_OK` upon success,
/// `vm_exec_result_t::WASMER_ERROR` otherwise. You can use
/// `wasmer_last_error_message()` to get the generated error message.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_set_breakpoint_value(
    instance_ptr: *const vm_exec_instance_t,
    value: u64,
) -> vm_exec_result_t {
    let capi_instance = cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null");
    capi_instance.content.set_breakpoint_value(value);
    vm_exec_result_t::VM_EXEC_OK
}

/// Gets the size in bytes of the memory data.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_get_breakpoint_value(
    instance_ptr: *const vm_exec_instance_t,
) -> u64 {
    let capi_instance =
        cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null", 0);
    capi_instance.content.get_breakpoint_value()
}
