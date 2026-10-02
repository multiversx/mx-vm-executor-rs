//! Instance lifecycle and the outer instance operations.
//!
//! "Outer" is the side of the trait split that the VM sees from outside a host call:
//! calling, gas accounting, and module introspection. Everything a *hook* needs is in
//! `capi_instance_state`, reached through a hook context rather than through this handle.
//! Keeping the split is what makes the Go side match the Rust trait pair instead of merely
//! wrapping it.

use libc::{c_char, c_int};
use meta::capi_safe_unwind;
use multiversx_chain_vm_executor::{BreakpointValue, Instance, InstanceCallResult};
use multiversx_chain_vm_executor_wasmer_experimental::ExperimentalInstance;
use std::ffi::CStr;
use std::slice;

use crate::capi_config::vm_exec2_compilation_options_t;
use crate::capi_executor::{CapiExecutor, vm_exec2_executor_t};
use crate::capi_status::{vm_exec2_status_t, write_error};
use crate::capi_strings::{copy_pipe_joined, pipe_joined_length};
use crate::capi_vm_hook_context::CapiVMHooksBuilder;

/// Opaque handle to an instance.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct vm_exec2_instance_t;

pub struct CapiInstance {
    pub(crate) content: ExperimentalInstance,
}

/// How a call ended.
///
/// This replaces the legacy arrangement, where a call returned a bare ok/error and the VM
/// then polled a wasm global to find out what had actually happened.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum vm_exec2_call_result_kind_t {
    /// The function returned normally.
    VM_EXEC2_CALL_OK = 0,

    /// No exported function by that name.
    VM_EXEC2_CALL_FUNCTION_NOT_FOUND = 1,

    /// The instance trapped for a reason that is neither a VM hook's doing nor a
    /// middleware breakpoint. The message is in the error buffer.
    VM_EXEC2_CALL_RUNTIME_ERROR = 2,

    /// A VM hook asked for execution to stop. `early_exit_code` says which; the message, if
    /// any, is in the error buffer, and is informational — the VM has already recorded its
    /// own.
    VM_EXEC2_CALL_EARLY_EXIT = 3,

    /// A middleware stopped execution. `breakpoint` says which.
    VM_EXEC2_CALL_BREAKPOINT = 4,
}

/// The out-struct of `vm_exec2_instance_call`.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct vm_exec2_call_result_t {
    pub kind: vm_exec2_call_result_kind_t,

    /// Meaningful only for `VM_EXEC2_CALL_EARLY_EXIT`.
    pub early_exit_code: u64,

    /// Meaningful only for `VM_EXEC2_CALL_BREAKPOINT`. Values match `BreakpointValue`:
    /// 4 = out of gas, 5 = memory limit.
    pub breakpoint: u64,
}

impl vm_exec2_call_result_t {
    fn of_kind(kind: vm_exec2_call_result_kind_t) -> Self {
        vm_exec2_call_result_t {
            kind,
            early_exit_code: 0,
            breakpoint: 0,
        }
    }
}

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
#[capi_safe_unwind(vm_exec2_status_t::VM_EXEC2_PANIC)]
pub unsafe extern "C" fn vm_exec2_new_instance(
    executor: *mut vm_exec2_executor_t,
    instance_out: *mut *mut vm_exec2_instance_t,
    wasm_bytes: *const u8,
    wasm_bytes_len: u32,
    options: *const vm_exec2_compilation_options_t,
    go_vm_hooks: usize,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec2_status_t {
    if executor.is_null() || instance_out.is_null() || wasm_bytes.is_null() || options.is_null() {
        return vm_exec2_status_t::VM_EXEC2_NULL_POINTER;
    }

    let capi_executor = unsafe { &*(executor as *const CapiExecutor) };
    let wasm_bytes = unsafe { slice::from_raw_parts(wasm_bytes, wasm_bytes_len as usize) };
    let compilation_options = unsafe { (*options).to_compilation_options() };

    if let Err(err) = multiversx_chain_vm_executor::check_missing_wasm(wasm_bytes) {
        return write_error(err, err_buf, err_buf_len);
    }

    let builder = CapiVMHooksBuilder::new(go_vm_hooks, capi_executor.hook_table.clone());

    let instance = match ExperimentalInstance::try_new_instance(
        Box::new(builder),
        capi_executor.opcode_config.clone(),
        wasm_bytes,
        &compilation_options,
    ) {
        Ok(instance) => instance,
        Err(err) => return write_error(err, err_buf, err_buf_len),
    };

    unsafe {
        *instance_out =
            Box::into_raw(Box::new(CapiInstance { content: instance })) as *mut vm_exec2_instance_t;
    }

    vm_exec2_status_t::VM_EXEC2_OK
}

/// Destroys an instance. Does nothing if `instance` is null.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vm_exec2_instance_destroy(instance: *mut vm_exec2_instance_t) {
    if !instance.is_null() {
        drop(unsafe { Box::from_raw(instance as *mut CapiInstance) });
    }
}

/// Calls an exported function, with a gas limit for this call.
///
/// Always writes `result_out` when it returns `VM_EXEC2_OK`. A non-`OK` status means the
/// call could not be attempted at all, which is distinct from a call that was attempted and
/// failed — the latter is reported through `result_out.kind`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec2_status_t::VM_EXEC2_PANIC)]
pub unsafe extern "C" fn vm_exec2_instance_call(
    instance: *mut vm_exec2_instance_t,
    func_name: *const c_char,
    points_limit: u64,
    result_out: *mut vm_exec2_call_result_t,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec2_status_t {
    if instance.is_null() || func_name.is_null() || result_out.is_null() {
        return vm_exec2_status_t::VM_EXEC2_NULL_POINTER;
    }

    let capi_instance = unsafe { &mut *(instance as *mut CapiInstance) };

    let Ok(func_name) = unsafe { CStr::from_ptr(func_name) }.to_str() else {
        return write_error("function name is not valid UTF-8", err_buf, err_buf_len);
    };

    let result = match capi_instance.content.call(func_name, points_limit) {
        InstanceCallResult::Ok => {
            vm_exec2_call_result_t::of_kind(vm_exec2_call_result_kind_t::VM_EXEC2_CALL_OK)
        }
        InstanceCallResult::FunctionNotFound => vm_exec2_call_result_t::of_kind(
            vm_exec2_call_result_kind_t::VM_EXEC2_CALL_FUNCTION_NOT_FOUND,
        ),
        InstanceCallResult::RuntimeError(err) => {
            write_error(err, err_buf, err_buf_len);
            vm_exec2_call_result_t::of_kind(
                vm_exec2_call_result_kind_t::VM_EXEC2_CALL_RUNTIME_ERROR,
            )
        }
        InstanceCallResult::VMHooksEarlyExit(early_exit) => {
            // Informational: the hook has already recorded its own message on the VM side.
            write_error(&early_exit.message, err_buf, err_buf_len);
            vm_exec2_call_result_t {
                kind: vm_exec2_call_result_kind_t::VM_EXEC2_CALL_EARLY_EXIT,
                early_exit_code: early_exit.code,
                breakpoint: 0,
            }
        }
        InstanceCallResult::Breakpoint(breakpoint) => vm_exec2_call_result_t {
            kind: vm_exec2_call_result_kind_t::VM_EXEC2_CALL_BREAKPOINT,
            early_exit_code: 0,
            breakpoint: breakpoint_as_u64(breakpoint),
        },
    };

    unsafe {
        *result_out = result;
    }

    vm_exec2_status_t::VM_EXEC2_OK
}

fn breakpoint_as_u64(breakpoint: BreakpointValue) -> u64 {
    breakpoint.as_u64()
}

/// Returns the number of points (gas) used so far.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec2_status_t::VM_EXEC2_PANIC)]
pub unsafe extern "C" fn vm_exec2_instance_get_points_used(
    instance: *mut vm_exec2_instance_t,
    points_out: *mut u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec2_status_t {
    if instance.is_null() || points_out.is_null() {
        return vm_exec2_status_t::VM_EXEC2_NULL_POINTER;
    }

    let capi_instance = unsafe { &mut *(instance as *mut CapiInstance) };
    match capi_instance.content.get_points_used() {
        Ok(points) => {
            unsafe {
                *points_out = points;
            }
            vm_exec2_status_t::VM_EXEC2_OK
        }
        Err(err) => write_error(err, err_buf, err_buf_len),
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
#[capi_safe_unwind(vm_exec2_status_t::VM_EXEC2_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec2_instance_check_signatures(
    instance: *const vm_exec2_instance_t,
) -> c_int {
    if instance.is_null() {
        return vm_exec2_status_t::VM_EXEC2_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*(instance as *const CapiInstance) };
    c_int::from(capi_instance.content.check_signatures())
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
#[capi_safe_unwind(vm_exec2_status_t::VM_EXEC2_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec2_instance_has_function(
    instance: *const vm_exec2_instance_t,
    func_name: *const c_char,
) -> c_int {
    if instance.is_null() || func_name.is_null() {
        return vm_exec2_status_t::VM_EXEC2_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*(instance as *const CapiInstance) };
    let Ok(func_name) = unsafe { CStr::from_ptr(func_name) }.to_str() else {
        return vm_exec2_status_t::VM_EXEC2_ERROR.as_c_int();
    };

    c_int::from(capi_instance.content.has_function(func_name))
}

/// Buffer length needed by `vm_exec2_instance_exported_function_names`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec2_status_t::VM_EXEC2_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec2_instance_exported_function_names_length(
    instance: *const vm_exec2_instance_t,
) -> c_int {
    if instance.is_null() {
        return vm_exec2_status_t::VM_EXEC2_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*(instance as *const CapiInstance) };
    pipe_joined_length(&capi_instance.content.get_exported_function_names())
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
#[capi_safe_unwind(vm_exec2_status_t::VM_EXEC2_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec2_instance_exported_function_names(
    instance: *const vm_exec2_instance_t,
    dest: *mut c_char,
    dest_len: c_int,
) -> c_int {
    if instance.is_null() {
        return vm_exec2_status_t::VM_EXEC2_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*(instance as *const CapiInstance) };
    unsafe {
        copy_pipe_joined(
            &capi_instance.content.get_exported_function_names(),
            dest,
            dest_len,
        )
    }
}

/// Buffer length needed by `vm_exec2_instance_imported_function_names`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec2_status_t::VM_EXEC2_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec2_instance_imported_function_names_length(
    instance: *const vm_exec2_instance_t,
) -> c_int {
    if instance.is_null() {
        return vm_exec2_status_t::VM_EXEC2_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*(instance as *const CapiInstance) };
    pipe_joined_length(&capi_instance.content.get_imported_function_names())
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
#[capi_safe_unwind(vm_exec2_status_t::VM_EXEC2_PANIC.as_c_int())]
pub unsafe extern "C" fn vm_exec2_instance_imported_function_names(
    instance: *const vm_exec2_instance_t,
    dest: *mut c_char,
    dest_len: c_int,
) -> c_int {
    if instance.is_null() {
        return vm_exec2_status_t::VM_EXEC2_NULL_POINTER.as_c_int();
    }

    let capi_instance = unsafe { &*(instance as *const CapiInstance) };
    unsafe {
        copy_pipe_joined(
            &capi_instance.content.get_imported_function_names(),
            dest,
            dest_len,
        )
    }
}
