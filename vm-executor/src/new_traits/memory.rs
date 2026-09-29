use std::{error::Error, fmt};

use crate::{MemLength, MemPtr};

/// The distinct ways an access to instance memory can fail.
///
/// The variants are the error cases the high-level VMs tell apart, so that a host can map
/// them onto its own error values by matching, rather than by parsing a message. The
/// messages match the ones the Go VM produces for the same conditions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    /// The requested offset lies outside the instance memory.
    BadBounds,

    /// The requested offset lies before the start of the instance memory.
    BadBoundsLower,

    /// The requested range ends past the end of the instance memory.
    BadBoundsUpper,

    /// The requested length is negative.
    NegativeLength,
}

impl MemoryError {
    pub fn message(self) -> &'static str {
        match self {
            MemoryError::BadBounds => "bad bounds",
            MemoryError::BadBoundsLower => "bad bounds (lower)",
            MemoryError::BadBoundsUpper => "bad bounds (upper)",
            MemoryError::NegativeLength => "negative length",
        }
    }
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl Error for MemoryError {}

/// Validates a range for a **strict** load, returning it as a `start..end` byte range.
///
/// The offset and length checks, and the errors they produce, mirror the Go VM's
/// `MemLoadFromMemory`. Only the end-of-range check differs: where the Go VM truncates the
/// range, this rejects it. Loads that need the truncating behaviour go through
/// [`crate::InstanceState::memory_load_clamped`], which is built on top of this.
///
/// A zero length yields an empty range, without validating the offset, because that is what
/// the Go VM does.
///
/// Shared by all the executor backends on purpose: the bounds rules are observable by
/// contracts, so there is one implementation of them and not one per backend.
pub fn checked_load_range(
    mem_ptr: MemPtr,
    mem_length: MemLength,
    memory_size: u64,
) -> Result<(usize, usize), MemoryError> {
    if mem_length == 0 {
        return Ok((0, 0));
    }
    if mem_ptr < 0 || mem_ptr as u64 > memory_size {
        return Err(MemoryError::BadBounds);
    }
    if mem_length < 0 {
        return Err(MemoryError::NegativeLength);
    }

    let end = (mem_ptr as u64)
        .checked_add(mem_length as u64)
        .ok_or(MemoryError::BadBoundsUpper)?;
    if end > memory_size {
        return Err(MemoryError::BadBoundsUpper);
    }

    Ok((mem_ptr as usize, end as usize))
}

/// Validates a range for a store, returning it as a `start..end` byte range.
///
/// Mirrors the Go VM's `MemStoreToMemory`, including the fact that an empty write is
/// accepted without validating the offset.
///
/// Shared by all the executor backends on purpose: the bounds rules are observable by
/// contracts, so there is one implementation of them and not one per backend.
pub fn checked_store_range(
    mem_ptr: MemPtr,
    data_len: usize,
    memory_size: u64,
) -> Result<(usize, usize), MemoryError> {
    if data_len == 0 {
        return Ok((0, 0));
    }
    if mem_ptr < 0 {
        return Err(MemoryError::BadBoundsLower);
    }

    let end = (mem_ptr as u64)
        .checked_add(data_len as u64)
        .ok_or(MemoryError::BadBoundsUpper)?;
    if end > memory_size {
        return Err(MemoryError::BadBoundsUpper);
    }

    Ok((mem_ptr as usize, end as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: u64 = 100;

    #[test]
    fn load_range_ok() {
        assert_eq!(checked_load_range(0, 100, SIZE), Ok((0, 100)));
        assert_eq!(checked_load_range(90, 10, SIZE), Ok((90, 100)));
        assert_eq!(checked_load_range(100, 0, SIZE), Ok((0, 0)));
    }

    #[test]
    fn load_range_zero_length_ignores_a_bad_offset() {
        // Matches the Go VM, which returns an empty slice before looking at the offset.
        assert_eq!(checked_load_range(-1, 0, SIZE), Ok((0, 0)));
        assert_eq!(checked_load_range(1_000, 0, SIZE), Ok((0, 0)));
    }

    #[test]
    fn load_range_bad_offset() {
        assert_eq!(
            checked_load_range(-1, 10, SIZE),
            Err(MemoryError::BadBounds)
        );
        assert_eq!(
            checked_load_range(101, 10, SIZE),
            Err(MemoryError::BadBounds)
        );
    }

    #[test]
    fn load_range_negative_length() {
        assert_eq!(
            checked_load_range(0, -1, SIZE),
            Err(MemoryError::NegativeLength)
        );
        // An out-of-range offset takes precedence over a negative length, as in the Go VM.
        assert_eq!(
            checked_load_range(-1, -1, SIZE),
            Err(MemoryError::BadBounds)
        );
    }

    #[test]
    fn load_range_end_past_memory() {
        assert_eq!(
            checked_load_range(95, 10, SIZE),
            Err(MemoryError::BadBoundsUpper)
        );
        // An offset exactly at the end is a valid offset, but nothing can be read from it.
        assert_eq!(
            checked_load_range(100, 1, SIZE),
            Err(MemoryError::BadBoundsUpper)
        );
    }

    #[test]
    fn load_range_does_not_overflow() {
        assert_eq!(
            checked_load_range(1, MemLength::MAX, SIZE),
            Err(MemoryError::BadBoundsUpper)
        );
    }

    #[test]
    fn store_range_ok() {
        assert_eq!(checked_store_range(0, 100, SIZE), Ok((0, 100)));
        assert_eq!(checked_store_range(90, 10, SIZE), Ok((90, 100)));
    }

    #[test]
    fn store_range_empty_data_ignores_a_bad_offset() {
        // Matches the Go VM, which returns before looking at the offset.
        assert_eq!(checked_store_range(-1, 0, SIZE), Ok((0, 0)));
    }

    #[test]
    fn store_range_bad_bounds() {
        assert_eq!(
            checked_store_range(-1, 10, SIZE),
            Err(MemoryError::BadBoundsLower)
        );
        assert_eq!(
            checked_store_range(95, 10, SIZE),
            Err(MemoryError::BadBoundsUpper)
        );
        // Unlike a load, a store past the end reports the upper bound, not `BadBounds`.
        assert_eq!(
            checked_store_range(1_000, 10, SIZE),
            Err(MemoryError::BadBoundsUpper)
        );
    }

    #[test]
    fn store_range_does_not_overflow() {
        assert_eq!(
            checked_store_range(1, usize::MAX, SIZE),
            Err(MemoryError::BadBoundsUpper)
        );
    }
}
