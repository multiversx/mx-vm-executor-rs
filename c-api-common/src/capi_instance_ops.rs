//! Instance lifecycle and the outer instance operations, shared by every backend.
//!
//! "Outer" is the side of the trait split that the VM sees from outside a host call:
//! calling, gas accounting, and module introspection. Everything a *hook* needs is in
//! `capi_state_ops`, reached through a hook context rather than through this handle.
//!
//! Nothing here takes a backend. The executor handle is already backend-specific, so
//! [`new_instance`] reads the choice off it, and everything after that goes through
//! `dyn Instance`.

use libc::{c_char, c_int};
use multiversx_chain_vm_executor::{CompilationOptions, Executor, ExecutorError, Instance};
use std::ffi::CStr;
use std::sync::Arc;
use std::{ptr, slice};

use crate::capi_call_result::vm_exec_call_result_t;
use crate::capi_config::vm_exec_compilation_options_t;
use crate::capi_handles::{CapiExecutor, CapiInstance};
use crate::capi_status::{vm_exec_status_t, write_error};
use crate::capi_strings::{copy_pipe_joined, pipe_joined_length};
use crate::capi_vm_hook_context::CapiVMHooksBuilder;

/// Creates an instance, using the backend the executor handle was created for.
///
/// # Safety
///
/// All pointer arguments must be valid for the duration of the call, and whatever
/// `go_vm_hooks` identifies must outlive the instance.
#[allow(clippy::too_many_arguments)]
pub unsafe fn new_instance(
    executor: *mut CapiExecutor,
    instance_out: *mut *mut CapiInstance,
    wasm_bytes: *const u8,
    wasm_bytes_len: u32,
    options: *const vm_exec_compilation_options_t,
    go_vm_hooks: usize,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe {
        build_instance(
            executor,
            instance_out,
            wasm_bytes,
            wasm_bytes_len,
            options,
            go_vm_hooks,
            err_buf,
            err_buf_len,
            |backend, bytes, options| backend.new_instance(bytes, options),
        )
    }
}

/// Creates an instance from a previously cached artifact, skipping compilation.
///
/// `cache_bytes` is what [`instance_cache`] produced, for the same contract and the same
/// backend. It is **executable code, not a module to validate**: see
/// `ExperimentalInstance::try_new_instance_from_cache` for what the backend does and does not
/// check. A caller that cannot vouch for the artifact's origin must use [`new_instance`] on
/// the wasm instead.
///
/// An artifact from a different Wasmer version is rejected rather than misread — its header
/// carries an ABI version — so an upgrade shows up as an error here, not as a wrong answer.
///
/// # Safety
///
/// All pointer arguments must be valid for the duration of the call, and whatever
/// `go_vm_hooks` identifies must outlive the instance.
#[allow(clippy::too_many_arguments)]
pub unsafe fn new_instance_from_cache(
    executor: *mut CapiExecutor,
    instance_out: *mut *mut CapiInstance,
    cache_bytes: *const u8,
    cache_bytes_len: u32,
    options: *const vm_exec_compilation_options_t,
    go_vm_hooks: usize,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe {
        build_instance(
            executor,
            instance_out,
            cache_bytes,
            cache_bytes_len,
            options,
            go_vm_hooks,
            err_buf,
            err_buf_len,
            |backend, bytes, options| backend.new_instance_from_cache(bytes, options),
        )
    }
}

/// The half the two creation entry points share: everything except which `Executor` method
/// turns the input bytes into an instance.
///
/// # Safety
///
/// As [`new_instance`].
#[allow(clippy::too_many_arguments)]
unsafe fn build_instance(
    executor: *mut CapiExecutor,
    instance_out: *mut *mut CapiInstance,
    bytes: *const u8,
    bytes_len: u32,
    options: *const vm_exec_compilation_options_t,
    go_vm_hooks: usize,
    err_buf: *mut c_char,
    err_buf_len: c_int,
    build: impl FnOnce(
        &dyn Executor,
        &[u8],
        &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError>,
) -> vm_exec_status_t {
    if executor.is_null() || instance_out.is_null() || bytes.is_null() || options.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER;
    }

    let capi_executor = unsafe { &*executor };
    let bytes = unsafe { slice::from_raw_parts(bytes, bytes_len as usize) };
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
    let backend_executor = (capi_executor.new_backend_executor)(runtime);

    let instance = match build(backend_executor.as_ref(), bytes, &compilation_options) {
        Ok(instance) => instance,
        Err(err) => return write_error(err, err_buf, err_buf_len),
    };

    unsafe {
        *instance_out = Box::into_raw(Box::new(CapiInstance::new(instance)));
    }

    vm_exec_status_t::VM_EXEC_OK
}

/// Serializes the instance's module and reports the buffer size [`instance_cache`] needs.
///
/// This is the first half of the usual two-call read, but it does the work rather than just
/// measuring it: the artifact is produced here and held on the instance for the copy call,
/// because serializing a few hundred KB twice to answer the same question is not worth the
/// symmetry with the function-name readers. The artifact stays valid until the instance is
/// destroyed or another length call replaces it.
///
/// # Safety
///
/// All pointer arguments must be valid for the duration of the call.
pub unsafe fn instance_cache_length(
    instance: *mut CapiInstance,
    length_out: *mut u32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    if instance.is_null() || length_out.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER;
    }

    let capi_instance = unsafe { &mut *instance };

    let artifact = match capi_instance.content.cache() {
        Ok(artifact) => artifact,
        Err(err) => return write_error(err, err_buf, err_buf_len),
    };

    unsafe {
        *length_out = artifact.len() as u32;
    }
    capi_instance.cached_artifact = Some(artifact);

    vm_exec_status_t::VM_EXEC_OK
}

/// Copies the artifact produced by the preceding [`instance_cache_length`] call.
///
/// Calling this without a preceding length call is an error rather than a silent empty
/// result, because the alternative — serializing here too — would make the cost of the pair
/// depend on the order the caller happened to use.
///
/// `dest_len` must be at least the length that call reported; a short buffer is refused, not
/// truncated, since half an artifact would deserialize to nothing useful and the caller has
/// no way to notice.
///
/// # Safety
///
/// `instance` must be valid, and `dest` must be writable for `dest_len` bytes.
pub unsafe fn instance_cache(
    instance: *mut CapiInstance,
    dest: *mut u8,
    dest_len: u32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    if instance.is_null() || dest.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER;
    }

    let capi_instance = unsafe { &mut *instance };

    let Some(artifact) = capi_instance.cached_artifact.as_ref() else {
        return write_error(
            "no cached artifact; call the cache length function first",
            err_buf,
            err_buf_len,
        );
    };

    if (dest_len as usize) < artifact.len() {
        return write_error(
            format!(
                "destination buffer of {} bytes is too small for a {}-byte artifact",
                dest_len,
                artifact.len()
            ),
            err_buf,
            err_buf_len,
        );
    }

    unsafe {
        ptr::copy_nonoverlapping(artifact.as_ptr(), dest, artifact.len());
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
