use crate::capi_executor::{vm_exec_executor_t, CapiExecutor};
use crate::capi_instance::{vm_exec_instance_t, CapiInstance};
use crate::service_singleton::with_service;
use crate::vm_exec_result_t;
use elrond_exec_service::OpcodeCost;

#[repr(C)]
pub struct vm_exec_opcode_cost_t;

// TODO: add comments
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_set_opcode_costs(
    executor_ptr: *mut vm_exec_executor_t,
    opcode_cost_ptr: *const vm_exec_opcode_cost_t,
) -> vm_exec_result_t {
    let capi_executor = cast_input_ptr!(executor_ptr, CapiExecutor, "executor ptr is null");
    let opcode_costs: &OpcodeCost = &*(opcode_cost_ptr as *const OpcodeCost);
    capi_executor.content.set_opcode_cost(opcode_costs);
    vm_exec_result_t::VM_EXEC_OK
}

// TODO: add comments
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_get_points_used(
    instance_ptr: *const vm_exec_instance_t,
) -> u64 {
    println!("vm_exec_instance_get_points_used");
    let capi_instance =
        cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null", 0);
    capi_instance.content.get_points_used()
}

// TODO: add comments
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_set_points_used(
    instance_ptr: *const vm_exec_instance_t,
    new_gas: u64,
) {
    println!("vm_exec_instance_set_points_used");
    if instance_ptr.is_null() {
        with_service(|service| service.update_last_error_str("instance ptr is null".to_string()));
        return;
    }
    let capi_instance = &*(instance_ptr as *const CapiInstance);
    capi_instance.content.set_points_used(new_gas);
}

// TODO: add comments
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_set_points_limit(
    instance_ptr: *mut vm_exec_instance_t,
    new_limit: u64,
) {
    println!("vm_exec_instance_set_points_limit");
    if instance_ptr.is_null() {
        with_service(|service| service.update_last_error_str("instance ptr is null".to_string()));
        return;
    }
    let capi_instance = &*(instance_ptr as *const CapiInstance);
    capi_instance.content.set_points_limit(new_limit);
}
