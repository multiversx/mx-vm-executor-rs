//! Executor lifecycle, shared by every backend's prefixed entry points.
//!
//! The functions here are the bodies; the `#[no_mangle]` wrappers live in the backend
//! crates, because cbindgen reads the source syntactically and only sees an entry point that
//! is written out literally. That is the whole reason for the split: the wrappers exist to be
//! *seen*, and carry the C contract as their documentation, while the logic is compiled once.

use libc::{c_char, c_int};
use std::sync::Arc;

use crate::capi_config::vm_exec_executor_config_t;
use crate::capi_handles::{CapiExecutor, NewBackendExecutor};
use crate::capi_status::{vm_exec_status_t, write_error};
use crate::capi_vm_hook_pointers::vm_exec_vm_hook_c_func_pointers;
use crate::capi_vm_hook_table::CapiVMHookTable;

/// Creates an executor from one configuration struct, one hook table and the backend the
/// calling library is for.
///
/// # Safety
///
/// `config` and `hook_table` must be valid for the duration of the call, and `executor_out`
/// must be writable. The hook table is copied, so the caller may release it on return.
pub unsafe fn new_executor(
    executor_out: *mut *mut CapiExecutor,
    config: *const vm_exec_executor_config_t,
    hook_table: *const vm_exec_vm_hook_c_func_pointers,
    new_backend_executor: NewBackendExecutor,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    if executor_out.is_null() || config.is_null() || hook_table.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER;
    }

    let opcode_config = match unsafe { (*config).to_opcode_config() } {
        Ok(opcode_config) => opcode_config,
        Err(message) => return write_error(message, err_buf, err_buf_len),
    };

    let capi_executor = CapiExecutor {
        opcode_config: Arc::new(opcode_config),
        hook_table: unsafe { CapiVMHookTable::copy_from(hook_table) },
        new_backend_executor,
    };

    unsafe {
        *executor_out = Box::into_raw(Box::new(capi_executor));
    }

    vm_exec_status_t::VM_EXEC_OK
}

/// Destroys an executor. Does nothing if `executor` is null.
///
/// # Safety
///
/// `executor` must be null, or a pointer this module produced and has not already destroyed.
pub unsafe fn executor_destroy(executor: *mut CapiExecutor) {
    if !executor.is_null() {
        drop(unsafe { Box::from_raw(executor) });
    }
}
