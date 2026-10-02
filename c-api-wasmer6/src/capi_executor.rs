//! Executor lifecycle.
//!
//! An executor here is the shared, immutable configuration: the opcode config and the Go
//! hook table. It is passed once, at creation, and never mutated — the legacy
//! `vm_exec_set_opcode_config` and `vm_exec_executor_set_vm_hooks_ptr` setters have no
//! counterpart.
//!
//! Note that it does not wrap a `dyn Executor`. The new `Executor` trait's `new_instance`
//! takes only the wasm bytes and the compilation options, with the hooks coming from the
//! executor's own runtime reference — but this API needs one Go hooks object *per instance*,
//! because that is where the Go side keeps the per-instance early-exit slot. So instances are
//! built directly against the Wasmer 6 backend, and the per-instance pointer is supplied at
//! `vm_exec_w6_new_instance`. Converging the two backends' runtime-reference traits (Phase 1.6)
//! should give the hooks provider that per-instance argument, at which point this can go back
//! through the trait and serve both backends.

use libc::{c_char, c_int};
use meta::capi_safe_unwind;
use multiversx_chain_vm_executor::OpcodeConfig;
use std::sync::Arc;

use crate::capi_config::vm_exec_w6_executor_config_t;
use crate::capi_status::{vm_exec_w6_status_t, write_error};
use crate::capi_vm_hook_pointers::vm_exec_w6_vm_hook_c_func_pointers;
use crate::capi_vm_hook_table::CapiVMHookTable;

/// Opaque handle to an executor.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct vm_exec_w6_executor_t;

pub struct CapiExecutor {
    pub(crate) opcode_config: Arc<OpcodeConfig>,
    pub(crate) hook_table: CapiVMHookTable,
}

/// Creates an executor from one configuration struct and one hook table.
///
/// `hook_table` is copied, so the caller is free to release it as soon as this returns. Every
/// entry must be populated; see `CapiVMHookTable::copy_from`.
///
/// On success writes the executor to `executor_out` and returns `VM_EXEC_W6_OK`. The caller
/// owns the result and must release it with `vm_exec_w6_executor_destroy`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_w6_status_t::VM_EXEC_W6_PANIC)]
pub unsafe extern "C" fn vm_exec_w6_new_executor(
    executor_out: *mut *mut vm_exec_w6_executor_t,
    config: *const vm_exec_w6_executor_config_t,
    hook_table: *const vm_exec_w6_vm_hook_c_func_pointers,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_w6_status_t {
    if executor_out.is_null() || config.is_null() || hook_table.is_null() {
        return vm_exec_w6_status_t::VM_EXEC_W6_NULL_POINTER;
    }

    let opcode_config = match unsafe { (*config).to_opcode_config() } {
        Ok(opcode_config) => opcode_config,
        Err(message) => return write_error(message, err_buf, err_buf_len),
    };

    let capi_executor = CapiExecutor {
        opcode_config: Arc::new(opcode_config),
        hook_table: unsafe { CapiVMHookTable::copy_from(hook_table) },
    };

    unsafe {
        *executor_out = Box::into_raw(Box::new(capi_executor)) as *mut vm_exec_w6_executor_t;
    }

    vm_exec_w6_status_t::VM_EXEC_W6_OK
}

/// Destroys an executor. Does nothing if `executor` is null.
///
/// Instances created from an executor do not borrow it, so the order of destruction between
/// the two does not matter.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vm_exec_w6_executor_destroy(executor: *mut vm_exec_w6_executor_t) {
    if !executor.is_null() {
        drop(unsafe { Box::from_raw(executor as *mut CapiExecutor) });
    }
}
