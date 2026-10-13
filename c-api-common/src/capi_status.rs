//! Status codes and the out-param error channel.
//!
//! The legacy C API reports errors through a `thread_local!` last-error singleton that the
//! caller reads back in a separate call. That does not survive the move to the new
//! interface: nothing pins a goroutine to the same OS thread between two cgo calls, so the
//! second call can land on a thread whose singleton holds someone else's error, or nothing
//! at all. Here every fallible entry point takes a caller-provided buffer instead, and
//! writes its message into it before returning.

use libc::{c_char, c_int};
use std::fmt::Display;
use std::{ptr, slice};

/// Result of a C API call.
///
/// `VM_EXEC_OK` is zero and every failure is negative, so the functions that return a
/// count (the memory loads) can share the same code space: a non-negative result is a
/// count, a negative one is one of these.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum vm_exec_status_t {
    /// The call succeeded.
    VM_EXEC_OK = 0,

    /// The call failed; the error message was written to the provided error buffer.
    VM_EXEC_ERROR = -1,

    /// A pointer argument that must not be null was null. No message is produced, because
    /// a null pointer may well be the error buffer itself.
    VM_EXEC_NULL_POINTER = -2,

    /// A hook context was used outside the host call it belongs to. See
    /// [`crate::capi_vm_hook_context`].
    VM_EXEC_STALE_CONTEXT = -3,

    /// A panic was caught at the boundary. Rust panics must never unwind into C, so every
    /// entry point traps them and reports this instead.
    VM_EXEC_PANIC = -4,
}

impl vm_exec_status_t {
    pub fn as_c_int(self) -> c_int {
        self as c_int
    }

    pub fn as_i64(self) -> i64 {
        self as i64
    }
}

/// Writes an error message into a caller-provided buffer.
///
/// The message is truncated to fit and always NUL-terminated. Truncation is silent and
/// deliberate: the alternative is a second call to ask for the length, which is the
/// round-trip this API is getting rid of. Callers size the buffer once; see
/// `VM_EXEC_ERROR_BUF_RECOMMENDED_LEN`.
///
/// A null buffer, or a buffer with no room for the terminator, is accepted and ignored, so
/// that a caller that does not care about the message can pass `(NULL, 0)`.
pub fn write_error(
    message: impl Display,
    err_buf: *mut c_char,
    err_buf_len: c_int,
) -> vm_exec_status_t {
    if !err_buf.is_null() && err_buf_len > 0 {
        let message = message.to_string();
        let capacity = err_buf_len as usize - 1; // room for the NUL

        // Truncated on a char boundary, so that the buffer always holds valid UTF-8 and the
        // Go side never has to deal with a split code point.
        let mut copy_len = message.len().min(capacity);
        while copy_len > 0 && !message.is_char_boundary(copy_len) {
            copy_len -= 1;
        }

        unsafe {
            let dest = slice::from_raw_parts_mut(err_buf as *mut u8, err_buf_len as usize);
            ptr::copy_nonoverlapping(message.as_ptr(), dest.as_mut_ptr(), copy_len);
            dest[copy_len] = 0;
        }
    }

    vm_exec_status_t::VM_EXEC_ERROR
}

/// Writes a message and returns it as a negative count, for the functions that return one.
pub fn write_error_count(message: impl Display, err_buf: *mut c_char, err_buf_len: c_int) -> i64 {
    write_error(message, err_buf, err_buf_len).as_i64()
}

/// A buffer of this size holds every message the executor currently produces.
///
/// Exported so the Go side has one place to take the number from rather than picking its
/// own.
#[unsafe(no_mangle)]
pub static VM_EXEC_ERROR_BUF_RECOMMENDED_LEN: c_int = 1024;

#[cfg(test)]
mod tests {
    use super::*;

    fn write_to_buf(message: &str, len: usize) -> (vm_exec_status_t, String) {
        let mut buf = vec![0i8; len];
        let status = write_error(message, buf.as_mut_ptr() as *mut c_char, len as c_int);
        let bytes: Vec<u8> = buf
            .iter()
            .take_while(|b| **b != 0)
            .map(|b| *b as u8)
            .collect();
        (status, String::from_utf8(bytes).expect("not valid UTF-8"))
    }

    #[test]
    fn error_message_round_trips() {
        let (status, written) = write_to_buf("boom", 64);
        assert_eq!(status, vm_exec_status_t::VM_EXEC_ERROR);
        assert_eq!(written, "boom");
    }

    #[test]
    fn error_message_is_truncated_to_fit() {
        let (_, written) = write_to_buf("0123456789", 5);
        assert_eq!(written, "0123");
    }

    #[test]
    fn error_message_truncation_keeps_valid_utf8() {
        // "é" is two bytes, so a 4-byte buffer holds "aé" and not a split code point.
        let (_, written) = write_to_buf("aéé", 4);
        assert_eq!(written, "aé");
    }

    #[test]
    fn error_message_tolerates_no_buffer() {
        assert_eq!(
            write_error("boom", std::ptr::null_mut(), 0),
            vm_exec_status_t::VM_EXEC_ERROR
        );
        assert_eq!(
            write_error("boom", std::ptr::null_mut(), 64),
            vm_exec_status_t::VM_EXEC_ERROR
        );
    }

    #[test]
    fn status_codes_leave_room_for_counts() {
        assert_eq!(vm_exec_status_t::VM_EXEC_OK.as_i64(), 0);
        for status in [
            vm_exec_status_t::VM_EXEC_ERROR,
            vm_exec_status_t::VM_EXEC_NULL_POINTER,
            vm_exec_status_t::VM_EXEC_STALE_CONTEXT,
            vm_exec_status_t::VM_EXEC_PANIC,
        ] {
            assert!(status.as_i64() < 0, "{status:?} must be negative");
        }
    }
}
