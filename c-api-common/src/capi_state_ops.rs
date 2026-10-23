//! The state operations: what a VM hook may do to the instance it is running inside.
//!
//! Every function here takes a hook context and is valid only while the hook that was handed
//! that context is still running. See `capi_vm_hook_context` for how that is enforced; a
//! stale context yields `VM_EXEC_STALE_CONTEXT` rather than a dereference of a dead borrow.
//!
//! Bounds checking lives on this side of the boundary, not in Go. The executor is the only
//! side that knows the real extent of the instance memory, and the rules are observable by
//! contracts, so there is one implementation of them — shared with the other backends in
//! `vm-executor/src/new_traits/memory.rs` — rather than one per caller.
//!
//! Nothing here mentions a backend, and nothing here needs to: the state arrives through the
//! context as an erased `&mut dyn InstanceState`. That is why these bodies are shared whole,
//! and each backend crate contributes only the prefixed symbols.

use libc::{c_char, c_int};
use multiversx_chain_vm_executor::{
    ExecutorError, MemLength, MemPtr, ServiceError, clamped_load_len,
};
use std::slice;

use crate::capi_status::{vm_exec_status_t, write_error, write_error_count};
use crate::capi_vm_hook_context::{
    ErasedInstanceState, vm_exec_hook_context_t, with_instance_state,
};

/// Loads from instance memory into a caller-provided buffer, truncating at the end of the
/// memory. Returns the number of bytes written, or a negative status.
///
/// # Safety
///
/// `ctx` must be null or a context this crate activated; `dest` must be writable for
/// `dest_len` bytes.
pub unsafe fn state_mem_load(
    ctx: *mut vm_exec_hook_context_t,
    mem_ptr: i32,
    dest: *mut u8,
    dest_len: i32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> i64 {
    let loaded = unsafe {
        with_instance_state::<ErasedInstanceState, _>(
            ctx,
            |state| -> Result<usize, ExecutorError> {
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
            },
        )
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
/// `ctx` must be null or a context this crate activated; `src` must be readable for
/// `src_len` bytes.
pub unsafe fn state_mem_store(
    ctx: *mut vm_exec_hook_context_t,
    mem_ptr: i32,
    src: *const u8,
    src_len: i32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    if src_len < 0 {
        return write_error("negative length", err_buf, err_buf_len);
    }
    if src.is_null() && src_len > 0 {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER;
    }

    let stored = unsafe {
        with_instance_state::<ErasedInstanceState, _>(ctx, |state| {
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
        Ok(Ok(())) => vm_exec_status_t::VM_EXEC_OK,
    }
}

/// Returns the size in bytes of the instance memory.
///
/// # Safety
///
/// `ctx` must be null or a context this crate activated; `length_out` must be writable.
pub unsafe fn state_mem_length(
    ctx: *mut vm_exec_hook_context_t,
    length_out: *mut u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    if length_out.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER;
    }

    let length = unsafe {
        with_instance_state::<ErasedInstanceState, _>(ctx, |state| state.memory_length())
    };

    match length {
        Err(context_err) => context_err.as_status(),
        Ok(Err(err)) => write_error(err, err_buf, err_buf_len),
        Ok(Ok(length)) => {
            unsafe {
                *length_out = length;
            }
            vm_exec_status_t::VM_EXEC_OK
        }
    }
}

/// Grows the instance memory by `by_num_pages` pages of 64Kb each.
///
/// # Safety
///
/// `ctx` must be null or a context this crate activated; `prev_pages_out` must be writable.
pub unsafe fn state_mem_grow(
    ctx: *mut vm_exec_hook_context_t,
    by_num_pages: u32,
    prev_pages_out: *mut u32,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    if prev_pages_out.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER;
    }

    let grown = unsafe {
        with_instance_state::<ErasedInstanceState, _>(ctx, |state| state.memory_grow(by_num_pages))
    };

    match grown {
        Err(context_err) => context_err.as_status(),
        Ok(Err(err)) => write_error(err, err_buf, err_buf_len),
        Ok(Ok(prev_pages)) => {
            unsafe {
                *prev_pages_out = prev_pages;
            }
            vm_exec_status_t::VM_EXEC_OK
        }
    }
}

/// Returns the number of points (gas) used by the instance so far.
///
/// # Safety
///
/// `ctx` must be null or a context this crate activated; `points_out` must be writable.
pub unsafe fn state_get_points_used(
    ctx: *mut vm_exec_hook_context_t,
    points_out: *mut u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    if points_out.is_null() {
        return vm_exec_status_t::VM_EXEC_NULL_POINTER;
    }

    let points = unsafe {
        with_instance_state::<ErasedInstanceState, _>(ctx, |state| state.get_points_used())
    };

    match points {
        Err(context_err) => context_err.as_status(),
        Ok(Err(err)) => write_error(err, err_buf, err_buf_len),
        Ok(Ok(points)) => {
            unsafe {
                *points_out = points;
            }
            vm_exec_status_t::VM_EXEC_OK
        }
    }
}

/// Sets the number of points (gas) used by the instance.
///
/// # Safety
///
/// `ctx` must be null or a context this crate activated.
pub unsafe fn state_set_points_used(
    ctx: *mut vm_exec_hook_context_t,
    points: u64,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    let set = unsafe {
        with_instance_state::<ErasedInstanceState, _>(ctx, |state| state.set_points_used(points))
    };

    match set {
        Err(context_err) => context_err.as_status(),
        Ok(Err(err)) => write_error(err, err_buf, err_buf_len),
        Ok(Ok(())) => vm_exec_status_t::VM_EXEC_OK,
    }
}
