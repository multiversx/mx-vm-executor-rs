
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
