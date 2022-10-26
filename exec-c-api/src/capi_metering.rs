use crate::capi_executor::{vm_exec_executor_t, CapiExecutor};
use crate::capi_instance::{vm_exec_instance_t, CapiInstance};
use crate::service_singleton::with_service;
use crate::vm_exec_result_t;
use elrond_exec_service::OpcodeCost;

#[allow(non_camel_case_types)]
#[repr(C)]
pub struct vm_exec_opcode_cost_t;

// vm_exec_set_opcode_costs sets the opcode costs for the given executor.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_set_opcode_costs(
    executor_ptr: *mut vm_exec_executor_t,
    opcode_cost_ptr: *const vm_exec_opcode_cost_t,
) -> vm_exec_result_t {
    println!("API: vm_exec_set_opcode_costs");
    let capi_executor = cast_input_ptr!(executor_ptr, CapiExecutor, "executor ptr is null");
    let opcode_costs: &OpcodeCost = &*(opcode_cost_ptr as *const OpcodeCost);
    capi_executor.content.set_opcode_cost(opcode_costs);
    vm_exec_result_t::VM_EXEC_OK
}

// vm_exec_instance_set_points_limit sets the number of points(gas) limit for the given instance.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_set_points_limit(
    instance_ptr: *mut vm_exec_instance_t,
    limit: u64,
) -> vm_exec_result_t {
    println!("API: vm_exec_instance_set_points_limit");
    let capi_instance = cast_input_ptr!(instance_ptr, CapiInstance, "instance ptr is null");
    capi_instance.content.set_points_limit(limit);
    vm_exec_result_t::VM_EXEC_OK
}

// vm_exec_instance_set_points_used sets the number of points(gas) for the given instance.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_set_points_used(
    instance_ptr: *mut vm_exec_instance_t,
    points: u64,
) -> vm_exec_result_t {
    println!("API: vm_exec_instance_set_points_used");
    let capi_instance = cast_input_ptr!(instance_ptr, CapiInstance, "instance ptr is null");
    capi_instance.content.set_points_used(points);
    vm_exec_result_t::VM_EXEC_OK
}

// vm_exec_instance_get_points_used returns the number of points(gas) used by the given instance.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_get_points_used(
    instance_ptr: *const vm_exec_instance_t,
) -> u64 {
    println!("API: vm_exec_instance_get_points_used");
    let capi_instance =
        cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null", 0);
    capi_instance.content.get_points_used()
}
