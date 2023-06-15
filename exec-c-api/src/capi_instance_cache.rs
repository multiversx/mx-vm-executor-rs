use std::slice;

use multiversx_chain_vm_executor::CompilationOptions;

use crate::{
    capi_executor::{vm_exec_executor_t, CapiExecutor},
    capi_instance::{vm_exec_compilation_options_t, vm_exec_instance_t, CapiInstance},
    service_singleton::with_service,
    vm_exec_result_t,
};

/// Caches an instance.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_cache(
    instance_ptr: *const vm_exec_instance_t,
    cache_bytes_ptr: *mut *const u8,
    cache_bytes_len: *mut u32,
) -> vm_exec_result_t {
    let capi_instance = cast_input_const_ptr!(instance_ptr, CapiInstance, "instance ptr is null");

    let result = capi_instance.content.cache();
    match result {
        Ok(bytes) => {
            *cache_bytes_ptr = bytes.as_ptr();
            *cache_bytes_len = bytes.len() as u32;
            std::mem::forget(bytes);
            vm_exec_result_t::VM_EXEC_OK
        }
        Err(message) => {
            with_service(|service| service.update_last_error_str(message));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}

/// Creates a new VM executor instance from cache.
///
/// All of the context comes from the provided VM executor.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment, unused_variables)]
#[no_mangle]
pub unsafe extern "C" fn vm_exec_instance_from_cache(
    executor_ptr: *mut vm_exec_executor_t,
    instance_ptr_ptr: *mut *mut vm_exec_instance_t,
    cache_bytes_ptr: *mut u8,
    cache_bytes_len: u32,
    options_ptr: *const vm_exec_compilation_options_t,
) -> vm_exec_result_t {
    let capi_executor = cast_input_ptr!(executor_ptr, CapiExecutor, "executor ptr is null");

    if cache_bytes_ptr.is_null() {
        with_service(|service| {
            service.update_last_error_str("cache bytes ptr is null".to_string())
        });
        return vm_exec_result_t::VM_EXEC_ERROR;
    }

    let cache_bytes: &[u8] = slice::from_raw_parts(cache_bytes_ptr, cache_bytes_len as usize);
    let compilation_options: &CompilationOptions = &*(options_ptr as *const CompilationOptions);
    let instance_result = capi_executor
        .content
        .new_instance_from_cache(cache_bytes, compilation_options);
    match instance_result {
        Ok(instance_box) => {
            let capi_instance = CapiInstance {
                content: instance_box,
            };
            *instance_ptr_ptr = Box::into_raw(Box::new(capi_instance)) as *mut vm_exec_instance_t;
            vm_exec_result_t::VM_EXEC_OK
        }
        Err(message) => {
            with_service(|service| service.update_last_error_str(message.to_string()));
            vm_exec_result_t::VM_EXEC_ERROR
        }
    }
}
