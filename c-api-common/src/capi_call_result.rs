//! How a call ended, as it crosses the boundary.
//!
//! This replaces the legacy arrangement, where a call returned a bare ok/error and the VM then
//! polled a wasm global to find out what had actually happened. The reason now travels with
//! the result.
//!
//! Shared between backends: an `InstanceCallResult` means the same thing whichever runtime
//! produced it, and both report it in the same struct so that a caller linking both sees one
//! definition.

use multiversx_chain_vm_executor::{BreakpointValue, InstanceCallResult};

use crate::capi_status::write_error;
use libc::{c_char, c_int};

/// How a call ended.
///
/// This replaces the legacy arrangement, where a call returned a bare ok/error and the VM
/// then polled a wasm global to find out what had actually happened.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum vm_exec_call_result_kind_t {
    /// The function returned normally.
    VM_EXEC_CALL_OK = 0,

    /// No exported function by that name.
    VM_EXEC_CALL_FUNCTION_NOT_FOUND = 1,

    /// The instance trapped for a reason that is neither a VM hook's doing nor a
    /// middleware breakpoint. The message is in the error buffer.
    VM_EXEC_CALL_RUNTIME_ERROR = 2,

    /// A VM hook asked for execution to stop. `early_exit_code` says which; the message, if
    /// any, is in the error buffer, and is informational — the VM has already recorded its
    /// own.
    VM_EXEC_CALL_EARLY_EXIT = 3,

    /// A middleware stopped execution. `breakpoint` says which.
    VM_EXEC_CALL_BREAKPOINT = 4,
}

/// The out-struct of `vm_exec_w7_instance_call`.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct vm_exec_call_result_t {
    pub kind: vm_exec_call_result_kind_t,

    /// Meaningful only for `VM_EXEC_CALL_EARLY_EXIT`.
    pub early_exit_code: u64,

    /// Meaningful only for `VM_EXEC_CALL_BREAKPOINT`. Values match `BreakpointValue`:
    /// 4 = out of gas, 5 = memory limit.
    pub breakpoint: u64,
}

impl vm_exec_call_result_t {
    pub fn of_kind(kind: vm_exec_call_result_kind_t) -> Self {
        vm_exec_call_result_t {
            kind,
            early_exit_code: 0,
            breakpoint: 0,
        }
    }
}

impl vm_exec_call_result_t {
    /// Converts a call outcome, writing any message the caller might want into `err_buf`.
    ///
    /// The message is informational in both reporting cases. For an early exit the VM has
    /// already recorded its own, which is why the text does not have to cross the boundary at
    /// all; what arrives here is whatever the executor had to say about it.
    ///
    /// # Safety
    ///
    /// `err_buf` must be null or writable for `err_buf_len` bytes.
    pub unsafe fn from_call_result(
        result: InstanceCallResult,
        err_buf: *mut c_char,
        err_buf_len: c_int,
    ) -> Self {
        match result {
            InstanceCallResult::Ok => {
                vm_exec_call_result_t::of_kind(vm_exec_call_result_kind_t::VM_EXEC_CALL_OK)
            }
            InstanceCallResult::FunctionNotFound => vm_exec_call_result_t::of_kind(
                vm_exec_call_result_kind_t::VM_EXEC_CALL_FUNCTION_NOT_FOUND,
            ),
            InstanceCallResult::RuntimeError(err) => {
                write_error(err, err_buf, err_buf_len);
                vm_exec_call_result_t::of_kind(
                    vm_exec_call_result_kind_t::VM_EXEC_CALL_RUNTIME_ERROR,
                )
            }
            InstanceCallResult::VMHooksEarlyExit(early_exit) => {
                write_error(&early_exit.message, err_buf, err_buf_len);
                vm_exec_call_result_t {
                    kind: vm_exec_call_result_kind_t::VM_EXEC_CALL_EARLY_EXIT,
                    early_exit_code: early_exit.code,
                    breakpoint: 0,
                }
            }
            InstanceCallResult::Breakpoint(breakpoint) => vm_exec_call_result_t {
                kind: vm_exec_call_result_kind_t::VM_EXEC_CALL_BREAKPOINT,
                early_exit_code: 0,
                breakpoint: breakpoint_as_u64(breakpoint),
            },
        }
    }
}

fn breakpoint_as_u64(breakpoint: BreakpointValue) -> u64 {
    breakpoint.as_u64()
}
