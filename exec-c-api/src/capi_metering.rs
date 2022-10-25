use crate::capi_instance::{vm_exec_instance_t, CapiInstance};
use crate::{service_singleton::with_service, vm_exec_result_t};

// TODO: add comments
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_instance_get_points_used(
    instance_ptr: *const vm_exec_instance_t,
) -> u64 {
    let _capi_instance =
        cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null", 0);
    0
}

// TODO: add comments
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_instance_set_remaining_points(
    instance_ptr: *mut vm_exec_instance_t,
    _new_gas: u64,
) -> vm_exec_result_t {
    let _capi_instance = cast_input_ptr!(instance_ptr, CapiInstance, "instance ptr is null");
    vm_exec_result_t::VM_EXEC_OK
}

// TODO: add comments
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn wasmer_instance_set_points_limit(
    instance_ptr: *mut vm_exec_instance_t,
    _limit: u64,
) -> vm_exec_result_t {
    let _capi_instance = cast_input_ptr!(instance_ptr, CapiInstance, "instance ptr is null");
    vm_exec_result_t::VM_EXEC_OK
}
