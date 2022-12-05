use crate::{
    capi_instance::{vm_exec_compilation_options_t, vm_exec_instance_t, CapiInstance},
    service_singleton::with_service,
    vm_exec_result_t,
};

/// Enable rkyv.
///
/// C API function
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_enable_rkyv() {}

/// Disable rkyv.
///
/// C API function
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_disable_rkyv() {}

/// Caches an instance.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment, unused_variables)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_cache(
    instance_ptr: *const vm_exec_instance_t,
    cache_bytes_ptr: *mut u8,
    cache_bytes_len: u32,
) -> vm_exec_result_t {
    let capi_instance = cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null");
    vm_exec_result_t::VM_EXEC_OK
}

/// Retrieves an instance from cache.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment, unused_variables)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_from_cache(
    instance_ptr_ptr: *mut *mut vm_exec_instance_t,
    cache_bytes_ptr: *mut u8,
    cache_bytes_len: u32,
    options_ptr: *const vm_exec_compilation_options_t,
) -> vm_exec_result_t {
    vm_exec_result_t::VM_EXEC_OK
}
