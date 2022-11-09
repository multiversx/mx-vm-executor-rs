use crate::capi_instance::{vm_exec_instance_t, CapiInstance};
use crate::service_singleton::with_service;
use crate::vm_exec_result_t;

//
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

//
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_get_breakpoint_value(
    instance_ptr: *const vm_exec_instance_t,
) -> u64 {
    let capi_instance =
        cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null", 0);
    capi_instance.content.get_breakpoint_value()
}
