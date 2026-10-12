//! The state operations: what a VM hook may do to the instance it is running inside.
//!
//! Every function here takes a hook context and is valid only while the hook that was handed
//! that context is still running. See `capi_vm_hook_context` for how that is enforced; a stale
//! context yields `VM_EXEC_STALE_CONTEXT` rather than a dereference of a dead borrow.
//!
//! Bounds checking lives on this side of the boundary, not in Go. The executor is the only
//! side that knows the real extent of the instance memory, and the rules are observable by
//! contracts, so there is one implementation of them — shared with the other backends in
//! `vm-executor/src/new_traits/memory.rs` — rather than one per caller.
//!
//! The bodies are in `c-api-common`, shared verbatim with the other backend's library: the
//! state arrives through the context as an erased `&mut dyn InstanceState`, so none of this
//! names a backend. What is here is the prefixed symbols and the C contract.

use libc::{c_char, c_int};
use meta::capi_safe_unwind;

use crate::capi_state_ops;
use crate::capi_status::vm_exec_status_t;
use crate::capi_vm_hook_context::vm_exec_hook_context_t;

/// Loads from instance memory into a caller-provided buffer, **truncating** at the end of
/// the memory.
///
/// Returns the number of bytes actually written, which may be fewer than `dest_len`, or a
/// negative status. The truncation is where the Go VM's historical load semantics live: a
/// load whose offset is valid but whose end lies past the end of the memory yields the bytes
/// that are available and no error. Contracts have observed that, so the executor keeps
/// offering it. An offset outside the memory, or a negative length, is still an error.
///
/// `dest_len` doubles as the requested length, which is what makes the clamping visible to
/// the caller: Go allocates `length` bytes and gets back `min(length, available)`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC.as_i64())]
pub unsafe extern "C" fn vm_exec_w7_state_mem_load(
    ctx: *mut vm_exec_hook_context_t,
    mem_ptr: i32,
    dest: *mut u8,
    dest_len: i32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> i64 {
    unsafe { capi_state_ops::state_mem_load(ctx, mem_ptr, dest, dest_len, err_buf, err_buf_len) }
}

/// Writes to instance memory. The range must fit entirely inside the memory.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_state_mem_store(
    ctx: *mut vm_exec_hook_context_t,
    mem_ptr: i32,
    src: *const u8,
    src_len: i32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe { capi_state_ops::state_mem_store(ctx, mem_ptr, src, src_len, err_buf, err_buf_len) }
}

/// Returns the size in bytes of the instance memory.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_state_mem_length(
    ctx: *mut vm_exec_hook_context_t,
    length_out: *mut u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe { capi_state_ops::state_mem_length(ctx, length_out, err_buf, err_buf_len) }
}

/// Grows the instance memory by `by_num_pages` pages of 64Kb each.
///
/// Writes the number of pages the memory had *before* growing to `prev_pages_out`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_state_mem_grow(
    ctx: *mut vm_exec_hook_context_t,
    by_num_pages: u32,
    prev_pages_out: *mut u32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe {
        capi_state_ops::state_mem_grow(ctx, by_num_pages, prev_pages_out, err_buf, err_buf_len)
    }
}

/// Returns the number of points (gas) used by the instance so far.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_state_get_points_used(
    ctx: *mut vm_exec_hook_context_t,
    points_out: *mut u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe { capi_state_ops::state_get_points_used(ctx, points_out, err_buf, err_buf_len) }
}

/// Sets the number of points (gas) used by the instance.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_status_t::VM_EXEC_PANIC)]
pub unsafe extern "C" fn vm_exec_w7_state_set_points_used(
    ctx: *mut vm_exec_hook_context_t,
    points: u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    unsafe { capi_state_ops::state_set_points_used(ctx, points, err_buf, err_buf_len) }
}
