//! The state operations: what a VM hook may do to the instance it is running inside.
//!
//! Every function here takes a hook context and is valid only while the hook that was handed
//! that context is still running. See `capi_vm_hook_context` for how that is enforced; a stale
//! context yields `VM_EXEC_W6_STALE_CONTEXT` rather than a dereference of a dead borrow.
//!
//! Bounds checking lives on this side of the boundary, not in Go. The executor is the only
//! side that knows the real extent of the instance memory, and the rules are observable by
//! contracts, so there is one implementation of them — shared with the other backends in
//! `vm-executor/src/new_traits/memory.rs` — rather than one per caller.

use libc::{c_char, c_int};
use meta::capi_safe_unwind;
use multiversx_chain_vm_executor::{
    ExecutorError, InstanceState, MemLength, MemPtr, ServiceError, clamped_load_len,
};
use std::slice;

use crate::capi_status::{vm_exec_w6_status_t, write_error, write_error_count};
use crate::capi_vm_hook_context::{ContextError, vm_exec_w6_hook_context_t, with_instance_state};

impl ContextError {
    fn as_status(self) -> vm_exec_w6_status_t {
        match self {
            ContextError::NullContext => vm_exec_w6_status_t::VM_EXEC_W6_NULL_POINTER,
            ContextError::Stale => vm_exec_w6_status_t::VM_EXEC_W6_STALE_CONTEXT,
        }
    }
}

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
#[capi_safe_unwind(vm_exec_w6_status_t::VM_EXEC_W6_PANIC.as_i64())]
pub unsafe extern "C" fn vm_exec_w6_state_mem_load(
    ctx: *mut vm_exec_w6_hook_context_t,
    mem_ptr: i32,
    dest: *mut u8,
    dest_len: i32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> i64 {
    let loaded = unsafe {
        with_instance_state(ctx, |state| -> Result<usize, ExecutorError> {
            let memory_length = state.memory_length()?;
            let to_load =
                clamped_load_len(mem_ptr as MemPtr, dest_len as MemLength, memory_length)?;
            if to_load == 0 {
                return Ok(0);
            }

            if dest.is_null() {
                return Err(Box::new(ServiceError::new("destination buffer is null")));
            }

            let dest = slice::from_raw_parts_mut(dest, to_load);
            state.memory_load_to_slice(mem_ptr as MemPtr, dest)?;
            Ok(to_load)
        })
    };

    match loaded {
        Err(context_err) => context_err.as_status().as_i64(),
        Ok(Err(err)) => write_error_count(err, err_buf, err_buf_len),
        Ok(Ok(to_load)) => to_load as i64,
    }
}

/// Writes to instance memory. The range must fit entirely inside the memory.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_w6_status_t::VM_EXEC_W6_PANIC)]
pub unsafe extern "C" fn vm_exec_w6_state_mem_store(
    ctx: *mut vm_exec_w6_hook_context_t,
    mem_ptr: i32,
    src: *const u8,
    src_len: i32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_w6_status_t {
    if src_len < 0 {
        return write_error("negative length", err_buf, err_buf_len);
    }
    if src.is_null() && src_len > 0 {
        return vm_exec_w6_status_t::VM_EXEC_W6_NULL_POINTER;
    }

    let stored = unsafe {
        with_instance_state(ctx, |state| {
            let data = if src_len == 0 {
                &[][..]
            } else {
                slice::from_raw_parts(src, src_len as usize)
            };
            state.memory_store(mem_ptr as MemPtr, data)
        })
    };

    match stored {
        Err(context_err) => context_err.as_status(),
        Ok(Err(err)) => write_error(err, err_buf, err_buf_len),
        Ok(Ok(())) => vm_exec_w6_status_t::VM_EXEC_W6_OK,
    }
}

/// Returns the size in bytes of the instance memory.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_w6_status_t::VM_EXEC_W6_PANIC)]
pub unsafe extern "C" fn vm_exec_w6_state_mem_length(
    ctx: *mut vm_exec_w6_hook_context_t,
    length_out: *mut u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_w6_status_t {
    if length_out.is_null() {
        return vm_exec_w6_status_t::VM_EXEC_W6_NULL_POINTER;
    }

    let length = unsafe { with_instance_state(ctx, |state| state.memory_length()) };

    match length {
        Err(context_err) => context_err.as_status(),
        Ok(Err(err)) => write_error(err, err_buf, err_buf_len),
        Ok(Ok(length)) => {
            unsafe {
                *length_out = length;
            }
            vm_exec_w6_status_t::VM_EXEC_W6_OK
        }
    }
}

/// Grows the instance memory by `by_num_pages` pages of 64Kb each.
///
/// Writes the number of pages the memory had *before* growing to `prev_pages_out`.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_w6_status_t::VM_EXEC_W6_PANIC)]
pub unsafe extern "C" fn vm_exec_w6_state_mem_grow(
    ctx: *mut vm_exec_w6_hook_context_t,
    by_num_pages: u32,
    prev_pages_out: *mut u32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_w6_status_t {
    if prev_pages_out.is_null() {
        return vm_exec_w6_status_t::VM_EXEC_W6_NULL_POINTER;
    }

    let grown = unsafe { with_instance_state(ctx, |state| state.memory_grow(by_num_pages)) };

    match grown {
        Err(context_err) => context_err.as_status(),
        Ok(Err(err)) => write_error(err, err_buf, err_buf_len),
        Ok(Ok(prev_pages)) => {
            unsafe {
                *prev_pages_out = prev_pages;
            }
            vm_exec_w6_status_t::VM_EXEC_W6_OK
        }
    }
}

/// Returns the number of points (gas) used by the instance so far.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_w6_status_t::VM_EXEC_W6_PANIC)]
pub unsafe extern "C" fn vm_exec_w6_state_get_points_used(
    ctx: *mut vm_exec_w6_hook_context_t,
    points_out: *mut u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_w6_status_t {
    if points_out.is_null() {
        return vm_exec_w6_status_t::VM_EXEC_W6_NULL_POINTER;
    }

    let points = unsafe { with_instance_state(ctx, |state| state.get_points_used()) };

    match points {
        Err(context_err) => context_err.as_status(),
        Ok(Err(err)) => write_error(err, err_buf, err_buf_len),
        Ok(Ok(points)) => {
            unsafe {
                *points_out = points;
            }
            vm_exec_w6_status_t::VM_EXEC_W6_OK
        }
    }
}

/// Sets the number of points (gas) used by the instance.
///
/// # Safety
///
/// C API function, works with raw object pointers.
#[unsafe(no_mangle)]
#[capi_safe_unwind(vm_exec_w6_status_t::VM_EXEC_W6_PANIC)]
pub unsafe extern "C" fn vm_exec_w6_state_set_points_used(
    ctx: *mut vm_exec_w6_hook_context_t,
    points: u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_w6_status_t {
    let set = unsafe { with_instance_state(ctx, |state| state.set_points_used(points)) };

    match set {
        Err(context_err) => context_err.as_status(),
        Ok(Err(err)) => write_error(err, err_buf, err_buf_len),
        Ok(Ok(())) => vm_exec_w6_status_t::VM_EXEC_W6_OK,
    }
}
