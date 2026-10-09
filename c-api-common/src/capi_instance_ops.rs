//! Instance lifecycle and the outer instance operations, shared by every backend.
//!
//! "Outer" is the side of the trait split that the VM sees from outside a host call:
//! calling, gas accounting, and module introspection. Everything a *hook* needs is in
//! `capi_state_ops`, reached through a hook context rather than through this handle.
//!
//! Only [`new_instance`] is parameterised by backend, and only to pick which `Executor` to
//! build. Everything after that goes through `dyn Instance`.

use libc::{c_char, c_int};
use std::ffi::CStr;
use std::slice;
use std::sync::Arc;

use crate::capi_call_result::vm_exec_call_result_t;
use crate::capi_config::vm_exec_compilation_options_t;
use crate::capi_handles::{CapiBackend, CapiExecutor, CapiInstance};
use crate::capi_status::{vm_exec_status_t, write_error};
use crate::capi_strings::{copy_pipe_joined, pipe_joined_length};
use crate::capi_vm_hook_context::CapiVMHooksBuilder;

/// Creates an instance of `B`'s backend.
///
/// # Safety
///
/// All pointer arguments must be valid for the duration of the call, and whatever
/// `go_vm_hooks` identifies must outlive the instance.
#[allow(clippy::too_many_arguments)]
pub unsafe fn new_instance<B: CapiBackend>(
    executor: *mut CapiExecutor,
    instance_out: *mut *mut CapiInstance,
    wasm_bytes: *const u8,
    wasm_bytes_len: u32,
    options: *const vm_exec_compilation_options_t,
    go_vm_hooks: usize,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    if executor.is_null() || instance_out.is_null() || wasm_bytes.is_null() || options.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER;
    }

    let capi_executor = unsafe { &*executor };
    let wasm_bytes = unsafe { slice::from_raw_parts(wasm_bytes, wasm_bytes_len as usize) };
    let compilation_options =
        unsafe { (*options).to_compilation_options(capi_executor.opcode_config.clone()) };

    // One executor per instance, for as long as the runtime handle is per instance: the
    // handle is what the executor is built around, so it cannot outlive the instance that
    // owns it. Building it here costs nothing — an executor holds only the runtime — and it
    // means this goes through the `Executor` trait rather than around it. When the handle
    // moves to `new_executor`, the executor moves to `CapiExecutor` with it and this becomes
    // a plain `new_instance` call.
    let runtime = Arc::new(CapiVMHooksBuilder::new(
        go_vm_hooks,
        capi_executor.hook_table.clone(),
    ));
    let backend_executor = B::new_executor(runtime);

    let instance = match backend_executor.new_instance(wasm_bytes, &compilation_options) {
        Ok(instance) => instance,
        Err(err) => return write_error(err, err_buf, err_buf_len),
    };

    unsafe {
        *instance_out = Box::into_raw(Box::new(CapiInstance { content: instance }));
    }

    vm_exec_status_t::VM_EXEC_OK
}

/// Destroys an instance. Does nothing if `instance` is null.
///
/// # Safety
///
/// `instance` must be null, or a pointer this module produced and has not already destroyed.
pub unsafe fn instance_destroy(instance: *mut CapiInstance) {
    if !instance.is_null() {
        drop(unsafe { Box::from_raw(instance) });
    }
}

/// Calls an exported function, with a gas limit for this call.
///
/// # Safety
///
/// All pointer arguments must be valid for the duration of the call.
pub unsafe fn instance_call(
    instance: *mut CapiInstance,
    func_name: *const c_char,
    points_limit: u64,
    result_out: *mut vm_exec_call_result_t,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    if instance.is_null() || func_name.is_null() || result_out.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER;
    }

    let capi_instance = unsafe { &mut *instance };

    let Ok(func_name) = unsafe { CStr::from_ptr(func_name) }.to_str() else {
        return write_error("function name is not valid UTF-8", err_buf, err_buf_len);
    };

    let result = unsafe {
        vm_exec_call_result_t::from_call_result(
            capi_instance.content.call(func_name, points_limit),
            err_buf,
            err_buf_len,
        )
    };

    unsafe {
        *result_out = result;
    }

    vm_exec_status_t::VM_EXEC_OK
}

/// Returns the number of points (gas) used so far.
///
/// # Safety
///
/// All pointer arguments must be valid for the duration of the call.
pub unsafe fn instance_get_points_used(
    instance: *mut CapiInstance,
    points_out: *mut u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    if instance.is_null() || points_out.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER;
    }

    let capi_instance = unsafe { &mut *instance };
    match capi_instance.content.get_points_used() {
        Ok(points) => {
            unsafe {
                *points_out = points;
            }
            vm_exec_status_t::VM_EXEC_OK
        }
        Err(err) => write_error(err, err_buf, err_buf_len),
    }
}

/// Checks that every exported function takes no arguments and returns nothing.
///
/// # Safety
///
/// `instance` must be null or valid for the duration of the call.
pub unsafe fn instance_check_signatures(instance: *const CapiInstance) -> c_int {
    if instance.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*instance };
    c_int::from(capi_instance.content.check_signatures())
}

/// Checks whether the contract exports a function with the given name.
///
/// # Safety
///
/// Both pointers must be null or valid for the duration of the call.
pub unsafe fn instance_has_function(
    instance: *const CapiInstance,
    func_name: *const c_char,
) -> c_int {
    if instance.is_null() || func_name.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*instance };
    let Ok(func_name) = unsafe { CStr::from_ptr(func_name) }.to_str() else {
        return vm_exec_status_t::VM_EXEC_ERROR.as_c_int();
    };

    c_int::from(capi_instance.content.has_function(func_name))
}

/// Buffer length needed by [`instance_exported_function_names`].
///
/// # Safety
///
/// `instance` must be null or valid for the duration of the call.
pub unsafe fn instance_exported_function_names_length(instance: *const CapiInstance) -> c_int {
    if instance.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*instance };
    pipe_joined_length(&capi_instance.content.get_exported_function_names())
}

/// Writes the exported function names, pipe-separated.
///
/// # Safety
///
/// `instance` must be null or valid, and `dest` must be writable for `dest_len` bytes.
pub unsafe fn instance_exported_function_names(
    instance: *const CapiInstance,
    dest: *mut c_char,
    dest_len: c_int,
) -> c_int {
    if instance.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*instance };
    unsafe {
        copy_pipe_joined(
            &capi_instance.content.get_exported_function_names(),
            dest,
            dest_len,
        )
    }
}

/// Buffer length needed by [`instance_imported_function_names`].
///
/// # Safety
///
/// `instance` must be null or valid for the duration of the call.
pub unsafe fn instance_imported_function_names_length(instance: *const CapiInstance) -> c_int {
    if instance.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*instance };
    pipe_joined_length(&capi_instance.content.get_imported_function_names())
}

/// Writes the names of the VM hooks the contract imports, pipe-separated.
///
/// # Safety
///
/// `instance` must be null or valid, and `dest` must be writable for `dest_len` bytes.
pub unsafe fn instance_imported_function_names(
    instance: *const CapiInstance,
    dest: *mut c_char,
    dest_len: c_int,
) -> c_int {
    if instance.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*instance };
    unsafe {
        copy_pipe_joined(
            &capi_instance.content.get_imported_function_names(),
            dest,
            dest_len,
        )
    }
}
