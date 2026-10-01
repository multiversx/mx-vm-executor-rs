//! The two-step string-out convention.
//!
//! A list of names is returned as a single pipe-separated string, in two calls: one for the
//! length, one for the bytes. Same convention as the legacy API, so the Go side's existing
//! helper shape carries over.

use libc::{c_char, c_int};
use std::{ptr, slice};

/// The length, including the NUL terminator, that `copy_pipe_joined` needs for `names`.
pub(crate) fn pipe_joined_length(names: &[String]) -> c_int {
    if names.is_empty() {
        return 0;
    }

    // One separator between each pair, plus the NUL: `len_sum + count` covers both.
    let len_sum: usize = names.iter().map(String::len).sum();
    (len_sum + names.len()) as c_int
}

/// Writes `names` into `dest` as a pipe-separated, NUL-terminated string.
///
/// Returns the number of bytes written including the terminator, or `-1` if `dest` is null
/// or too small. Callers size `dest` with [`pipe_joined_length`].
///
/// # Safety
///
/// `dest` must be null or writable for `dest_len` bytes.
pub(crate) unsafe fn copy_pipe_joined(
    names: &[String],
    dest: *mut c_char,
    dest_len: c_int,
) -> c_int {
    let joined = names.join("|");

    if dest.is_null() || dest_len <= 0 {
        return -1;
    }
    let dest_len = dest_len as usize;
    if joined.len() >= dest_len {
        return -1;
    }

    unsafe {
        let dest = slice::from_raw_parts_mut(dest as *mut u8, dest_len);
        ptr::copy_nonoverlapping(joined.as_ptr(), dest.as_mut_ptr(), joined.len());
        dest[joined.len()] = 0;
    }

    joined.len() as c_int + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn join_into(names: &[&str], buf_len: usize) -> (c_int, String) {
        let names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
        let mut buf = vec![0i8; buf_len];
        let written =
            unsafe { copy_pipe_joined(&names, buf.as_mut_ptr() as *mut c_char, buf_len as c_int) };
        let bytes: Vec<u8> = buf
            .iter()
            .take_while(|b| **b != 0)
            .map(|b| *b as u8)
            .collect();
        (written, String::from_utf8(bytes).unwrap())
    }

    #[test]
    fn length_accounts_for_separators_and_terminator() {
        let names = vec!["init".to_string(), "upgrade".to_string()];
        // "init|upgrade" is 12 bytes, plus the NUL.
        assert_eq!(pipe_joined_length(&names), 13);
        assert_eq!(pipe_joined_length(&[]), 0);
    }

    #[test]
    fn length_is_exactly_what_the_copy_needs() {
        let names = vec!["init".to_string(), "upgrade".to_string()];
        let needed = pipe_joined_length(&names) as usize;

        let (written, joined) = join_into(&["init", "upgrade"], needed);
        assert_eq!(written, needed as c_int);
        assert_eq!(joined, "init|upgrade");

        // One byte short must be refused rather than truncated.
        let (written, _) = join_into(&["init", "upgrade"], needed - 1);
        assert_eq!(written, -1);
    }

    #[test]
    fn refuses_a_missing_buffer() {
        let names = vec!["init".to_string()];
        assert_eq!(unsafe { copy_pipe_joined(&names, ptr::null_mut(), 64) }, -1);
    }
}
