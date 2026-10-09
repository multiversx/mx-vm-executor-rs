//! Executor lifecycle.
//!
//! An executor here is the shared, immutable configuration: the opcode config and the Go
//! hook table. It is passed once, at creation, and never mutated — the legacy
//! `vm_exec_set_opcode_config` and `vm_exec_executor_set_vm_hooks_ptr` setters have no
//! counterpart.
//!
//! Note that it does not yet wrap a `dyn Executor`. An executor is built around one runtime
//! reference, for its whole life, and this API's runtime handle is still supplied per
//! instance at `vm_exec_w2_new_instance` — so the executor is built there, per instance, and
//! this struct holds the configuration it will be built from. The handle becomes
//! per-executor, and the `dyn Executor` moves here, once the Go VM stops reaching wasm memory
//! through `executor.Instance`; see `PLAN-go-ffi.md`.
//!
//! The opcode config is held here for the same reason. On the Rust side it has moved into
//! `CompilationOptions`, and this API merges the two at instance creation rather than
//! changing the C struct ahead of the Go VM.
//!
//! The bodies are in `c-api-common`, shared with the Wasmer 2.2 library. What is here is the
//! prefixed symbol and the C contract.

use libc::{c_char, c_int};
use meta::capi_safe_unwind;

use crate::capi_config::vm_exec_executor_config_t;
use crate::capi_executor_ops;
use crate::capi_handles::CapiExecutor;
use crate::capi_status::vm_exec_status_t;
use crate::capi_vm_hook_pointers::vm_exec_vm_hook_c_func_pointers;

/// Opaque handle to an executor.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct vm_exec_w2_executor_t;

/// Creates an executor from one configuration struct and one hook table.
///
/// `hook_table` is copied, so the caller is free to release it as soon as this returns. Every
/// entry must be populated; see `CapiVMHookTable::copy_from`.
///
/// On success writes the executor to `executor_out` and returns `VM_EXEC_OK`. The caller
/// owns the result and must release it with `vm_exec_w2_executor_destroy`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[allow(clippy::cast_ptr_alignment)]
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w2_new_executor(
    executor_out: *mut *mut vm_exec_w2_executor_t,
    config: *const vm_exec_executor_config_t,
    hook_table: *const vm_exec_vm_hook_c_func_pointers,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe {
        capi_executor_ops::new_executor(
            executor_out as *mut *mut CapiExecutor,
            config,
            hook_table,
            err_buf,
            err_buf_len,
        )
    }
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
pub unsafe extern "C" fn vm_exec_w2_executor_destroy(executor: *mut vm_exec_w2_executor_t) {
    unsafe { capi_executor_ops::executor_destroy(executor as *mut CapiExecutor) }
}
