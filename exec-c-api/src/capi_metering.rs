use std::slice;

use crate::capi_instance::{vm_exec_instance_t, CapiInstance};
use crate::service_singleton::with_service;

pub const OPCODE_COUNT: usize = 448;
pub static mut OPCODE_COSTS: [u32; OPCODE_COUNT] = [0; OPCODE_COUNT];

// TODO: add comments
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_set_opcode_costs(opcode_costs_pointer: *const u32) {
    println!("\nvm_exec_set_opcode_costs");
    OPCODE_COSTS.copy_from_slice(slice::from_raw_parts(opcode_costs_pointer, OPCODE_COUNT));
    for (i, cost) in OPCODE_COSTS.iter().enumerate() {
        println!("opcode {} cost {}", i, cost);
    }
    println!("\n");
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
