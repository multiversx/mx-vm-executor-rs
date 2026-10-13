//! Instance lifecycle and the outer instance operations.
//!
//! "Outer" is the side of the trait split that the VM sees from outside a host call:
//! calling, gas accounting, and module introspection. Everything a *hook* needs is in
//! `capi_instance_state`, reached through a hook context rather than through this handle.
//! Keeping the split is what makes the Go side match the Rust trait pair instead of merely
//! wrapping it.
//!
//! The bodies are in `c-api-common`, shared with the other backend's library; `capi_backend`
//! is the only thing that differs. What is here is the prefixed symbols and the C contract.

use libc::{c_char, c_int};
use meta::capi_safe_unwind;

use crate::capi_call_result::vm_exec_call_result_t;
use crate::capi_config::vm_exec_compilation_options_t;
use crate::capi_executor::vm_exec_w7_executor_t;
use crate::capi_handles::{CapiExecutor, CapiInstance};
use crate::capi_instance_ops;
use crate::capi_status::vm_exec_status_t;

/// Opaque handle to an instance.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct vm_exec_w7_instance_t;

/// Creates an instance.
///
/// `go_vm_hooks` is the caller's VM hooks handle for *this* instance; it is handed back to
/// every VM hook as field 0 of the hook context, and is otherwise opaque here. Whatever it
/// refers to must stay alive for the instance's lifetime. From Go it is a `cgo.Handle`,
/// which is why it is an integer and not a pointer.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_new_instance(
    executor: *mut vm_exec_w7_executor_t,
    instance_out: *mut *mut vm_exec_w7_instance_t,
    wasm_bytes: *const u8,
    wasm_bytes_len: u32,
    options: *const vm_exec_compilation_options_t,
    go_vm_hooks: usize,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe {
        capi_instance_ops::new_instance(
            executor as *mut CapiExecutor,
            instance_out as *mut *mut CapiInstance,
            wasm_bytes,
            wasm_bytes_len,
            options,
            go_vm_hooks,
            err_buf,
            err_buf_len,
        )
    }
}

/// Creates an instance from a cached artifact, skipping compilation.
///
/// `cache_bytes` is what `vm_exec_w7_instance_cache` produced, from this same library. It is
/// **executable code that is mapped, not validated** — an artifact from an untrusted source
/// must not be passed here; compile the wasm instead. An artifact from a different Wasmer
/// version is refused, because its header carries an ABI version.
///
/// `go_vm_hooks` is as in `vm_exec_w7_new_instance`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_new_instance_from_cache(
    executor: *mut vm_exec_w7_executor_t,
    instance_out: *mut *mut vm_exec_w7_instance_t,
    cache_bytes: *const u8,
    cache_bytes_len: u32,
    options: *const vm_exec_compilation_options_t,
    go_vm_hooks: usize,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe {
        capi_instance_ops::new_instance_from_cache(
            executor as *mut CapiExecutor,
            instance_out as *mut *mut CapiInstance,
            cache_bytes,
            cache_bytes_len,
            options,
            go_vm_hooks,
            err_buf,
            err_buf_len,
        )
    }
}

/// Destroys an instance. Does nothing if `instance` is null.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vm_exec_w7_instance_destroy(instance: *mut vm_exec_w7_instance_t) {
    unsafe { capi_instance_ops::instance_destroy(instance as *mut CapiInstance) }
}

/// Calls an exported function, with a gas limit for this call.
///
/// Always writes `result_out` when it returns `VM_EXEC_OK`. A non-`OK` status means the
/// call could not be attempted at all, which is distinct from a call that was attempted and
/// failed — the latter is reported through `result_out.kind`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_instance_call(
    instance: *mut vm_exec_w7_instance_t,
    func_name: *const c_char,
    points_limit: u64,
    result_out: *mut vm_exec_call_result_t,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe {
        capi_instance_ops::instance_call(
            instance as *mut CapiInstance,
            func_name,
            points_limit,
            result_out,
            err_buf,
            err_buf_len,
        )
    }
}

/// Returns the number of points (gas) used so far.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_instance_get_points_used(
    instance: *mut vm_exec_w7_instance_t,
    points_out: *mut u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe {
        capi_instance_ops::instance_get_points_used(
            instance as *mut CapiInstance,
            points_out,
            err_buf,
            err_buf_len,
        )
    }
}

/// Checks that every exported function takes no arguments and returns nothing.
///
/// Returns 1 if so, 0 if not, or a negative status.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec_w7_instance_check_signatures(
    instance: *const vm_exec_w7_instance_t,
) -> c_int {
    unsafe { capi_instance_ops::instance_check_signatures(instance as *const CapiInstance) }
}

/// Checks whether the contract exports a function with the given name.
///
/// Returns 1 if so, 0 if not, or a negative status.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec_w7_instance_has_function(
    instance: *const vm_exec_w7_instance_t,
    func_name: *const c_char,
) -> c_int {
    unsafe { capi_instance_ops::instance_has_function(instance as *const CapiInstance, func_name) }
}

/// Buffer length needed by `vm_exec_w7_instance_exported_function_names`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec_w7_instance_exported_function_names_length(
    instance: *const vm_exec_w7_instance_t,
) -> c_int {
    unsafe {
        capi_instance_ops::instance_exported_function_names_length(instance as *const CapiInstance)
    }
}

/// Writes the exported function names, pipe-separated, e.g. `"init|endpoint1|endpoint2"`.
///
/// No ordering is guaranteed. Returns the number of bytes written including the terminator,
/// or `-1` if the buffer is absent or too small.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec_w7_instance_exported_function_names(
    instance: *const vm_exec_w7_instance_t,
    dest: *mut c_char,
    dest_len: c_int,
) -> c_int {
    unsafe {
        capi_instance_ops::instance_exported_function_names(
            instance as *const CapiInstance,
            dest,
            dest_len,
        )
    }
}

/// Buffer length needed by `vm_exec_w7_instance_imported_function_names`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec_w7_instance_imported_function_names_length(
    instance: *const vm_exec_w7_instance_t,
) -> c_int {
    unsafe {
        capi_instance_ops::instance_imported_function_names_length(instance as *const CapiInstance)
    }
}

/// Writes the names of the VM hooks the contract imports, pipe-separated.
///
/// All of them at once, rather than a per-name query, because the Go contract validator
/// checks the module against whole sets of allowed names.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec_w7_instance_imported_function_names(
    instance: *const vm_exec_w7_instance_t,
    dest: *mut c_char,
    dest_len: c_int,
) -> c_int {
    unsafe {
        capi_instance_ops::instance_imported_function_names(
            instance as *const CapiInstance,
            dest,
            dest_len,
        )
    }
}

/// Serializes the instance's module and reports how many bytes `vm_exec_w7_instance_cache`
/// will write.
///
/// The artifact is produced by this call and held until the copy call reads it, so the pair
/// serializes once rather than twice. Call this first; see the copy call.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_instance_cache_length(
    instance: *mut vm_exec_w7_instance_t,
    length_out: *mut u32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe {
        capi_instance_ops::instance_cache_length(
            instance as *mut CapiInstance,
            length_out,
            err_buf,
            err_buf_len,
        )
    }
}

/// Copies the artifact produced by the preceding `vm_exec_w7_instance_cache_length` call.
///
/// `dest_len` must be at least the length that call reported. A short buffer is refused
/// rather than filled partially, because a truncated artifact is indistinguishable from a
/// whole one until it fails to deserialize.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_instance_cache(
    instance: *mut vm_exec_w7_instance_t,
    dest: *mut u8,
    dest_len: u32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe {
        capi_instance_ops::instance_cache(
            instance as *mut CapiInstance,
            dest,
            dest_len,
            err_buf,
            err_buf_len,
        )
    }
}

/// Returns the instance to the state it was in immediately after creation, so it can serve
/// another execution.
///
/// Memory contents and length, every mutable global, and the metering and breakpoint globals
/// are all restored. The handle stays valid — the instance is rebuilt behind it, not replaced.
///
/// On failure the caller must destroy the instance rather than reuse it: an instance whose
/// reset failed still carries the previous execution's state.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_instance_reset(
    instance: *mut vm_exec_w7_instance_t,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe {
        capi_instance_ops::instance_reset(instance as *mut CapiInstance, err_buf, err_buf_len)
    }
}
