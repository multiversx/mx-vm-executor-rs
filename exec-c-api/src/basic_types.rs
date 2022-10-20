use libc::{c_char, c_int};
use std::{ptr, slice};

/// The `wasmer_result_t` enum is a type that represents either a
/// success, or a failure.
#[allow(non_camel_case_types)]
#[repr(C)]
pub enum vm_exec_result_t {
    /// Represents a success.
    VM_EXEC_OK = 1,

    /// Represents a failure.
    VM_EXEC_ERROR = 2,
}

#[repr(C)]
pub struct vm_exec_byte_array {
    pub bytes: *const u8,
    pub bytes_len: u32,
}

impl vm_exec_byte_array {
    /// Get the data as a slice
    pub unsafe fn as_slice<'a>(&self) -> &'a [u8] {
        get_slice_checked(self.bytes, self.bytes_len as usize)
    }

    /// Copy the data into an owned Vec
    pub unsafe fn as_vec(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.bytes_len as usize);
        out.extend_from_slice(self.as_slice());

        out
    }

    /// Read the data as a &str, returns an error if the string is not valid UTF8
    pub unsafe fn as_str<'a>(&self) -> Result<&'a str, std::str::Utf8Error> {
        std::str::from_utf8(self.as_slice())
    }
}

/// Gets a slice from a pointer and a length, returning an empty slice if the
/// pointer is null
#[inline]
pub(crate) unsafe fn get_slice_checked<'a, T>(ptr: *const T, len: usize) -> &'a [T] {
    if ptr.is_null() {
        &[]
    } else {
        std::slice::from_raw_parts(ptr, len)
    }
}

pub(crate) fn string_length(s: String) -> c_int {
    if s.is_empty() {
        0
    } else {
        s.len() as c_int + 1 // NULL terminator
    }
}

/// Copies a String to destination pointer, over the C API.
pub(crate) unsafe fn string_copy(
    s: String,
    dest_buffer: *mut c_char,
    dest_buffer_len: c_int,
) -> c_int {
    if dest_buffer.is_null() {
        // buffer pointer is null
        return -1;
    }

    let dest_buffer_len = dest_buffer_len as usize;

    if s.len() >= dest_buffer_len {
        // buffer is too small to hold the error message
        return -1;
    }

    let dest_buffer = slice::from_raw_parts_mut(dest_buffer as *mut u8, dest_buffer_len);

    ptr::copy_nonoverlapping(s.as_ptr(), dest_buffer.as_mut_ptr(), s.len());

    // Add a trailing null so people using the string as a `char *` don't
    // accidentally read into garbage.
    dest_buffer[s.len()] = 0;

    s.len() as c_int + 1
}

#[repr(C)]
pub struct vm_exec_byte_array_list {
    pub arrays: *const vm_exec_byte_array,
    pub arrays_len: u32,
}
