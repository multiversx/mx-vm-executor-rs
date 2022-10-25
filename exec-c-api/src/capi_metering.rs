use crate::capi_instance::{vm_exec_instance_t, CapiInstance};
use crate::{service_singleton::with_service, vm_exec_result_t};

// TODO: add comments
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_instance_get_points_used(
    instance_ptr: *const vm_exec_instance_t,
) -> u64 {
    let capi_instance =
        cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null", 0);
    capi_instance.content.get_points_used()
}

// TODO: add comments
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_instance_set_points_used(
    instance_ptr: *mut vm_exec_instance_t,
    new_gas: u64,
) {
    let capi_instance = cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null");
    capi_instance.content.set_points_used(new_gas);
}

// TODO: add comments
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_instance_set_points_limit(
    instance_ptr: *mut vm_exec_instance_t,
    new_limit: u64,
) {
    let capi_instance = cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null");
    capi_instance.content.set_points_limit(new_limit);
}
