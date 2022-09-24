//! Read runtime errors.

// use crate::service::with_service;
use libc::{c_char, c_int};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    ptr, slice,
};

use crate::service_singleton::with_service;

/// Gets the length in bytes of the last error if any.
///
/// This can be used to dynamically allocate a buffer with the correct number of
/// bytes needed to store a message.
#[no_mangle]
pub extern "C" fn vm_exec_last_error_length() -> c_int {
    string_length(get_last_error_string())
}

/// Gets the last error message if any into the provided buffer
/// `buffer` up to the given `length`.
///
/// The `length` parameter must be large enough to store the last
/// error message. Ideally, the value should come from
/// `wasmer_last_error_length()`.
///
/// The function returns the length of the string in bytes, `-1` if an
/// error occurs. Potential errors are:
///
///  * The buffer is a null pointer,
///  * The buffer is too smal to hold the error message.
///
/// Note: The error message always has a trailing null character.
/// ```
#[no_mangle]
pub unsafe extern "C" fn vm_exec_last_error_message(
    dest_buffer: *mut c_char,
    dest_buffer_len: c_int,
) -> c_int {
    string_copy(get_last_error_string(), dest_buffer, dest_buffer_len)
}

#[no_mangle]
pub extern "C" fn vm_exec_execution_info_length() -> c_int {
    string_length(get_execution_info())
}

#[no_mangle]
pub unsafe extern "C" fn vm_exec_execution_info_message(
    dest_buffer: *mut c_char,
    dest_buffer_len: c_int,
) -> c_int {
    string_copy(get_execution_info(), dest_buffer, dest_buffer_len)
}

fn get_last_error_string() -> String {
    with_service(|service| service.get_last_error_string())
}

fn get_execution_info() -> String {
    with_service(|service| service.get_execution_info())
}

fn string_length(s: String) -> c_int {
    if s.is_empty() {
        0
    } else {
        s.len() as c_int + 1 // NULL terminator
    }
}

/// Copies a String to destination pointer, over the C API.
unsafe fn string_copy(s: String, dest_buffer: *mut c_char, dest_buffer_len: c_int) -> c_int {
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

#[derive(Debug)]
pub struct CApiError {
    message: &'static str,
}

impl CApiError {
    pub fn new(message: &'static str) -> Self {
        Self { message }
    }
}

impl Display for CApiError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", &self.message)
    }
}

impl Error for CApiError {}
