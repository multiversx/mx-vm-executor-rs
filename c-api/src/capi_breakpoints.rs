use multiversx_chain_vm_executor::Instance;

use crate::capi_instance::{vm_exec_instance_t, CapiInstance};
use crate::service_singleton::with_service;
use crate::vm_exec_result_t;

/// Sets the runtime breakpoint value for the given instance.
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
    let result = set_breakpoint_value_u64(capi_instance.content.as_ref(), value);
    match result {
        Ok(()) => vm_exec_result_t::VM_EXEC_OK,
        Err(message) => {
            with_service(|service| service.update_last_error_str(message));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

/// Returns the runtime breakpoint value from the given instance.
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
    let result = capi_instance.content.get_breakpoint_value();
    match result {
        Ok(breakpoint_value) => breakpoint_value.as_u64(),
        Err(message) => {
            with_service(|service| service.update_last_error_str(message));
            0
        }
    }
}

fn set_breakpoint_value_u64(instance: &dyn Instance, value: u64) -> Result<(), String> {
    instance.set_breakpoint_value(value.try_into()?)
}
