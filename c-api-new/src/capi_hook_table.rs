//! The Go VM hook function-pointer table, owned by Rust.
//!
//! The table is **copied** at executor creation rather than referenced, exactly as the
//! legacy API does. Holding the caller's pointer instead would make the executor's validity
//! depend on the caller keeping a Go-side struct alive and unmoved for the executor's whole
//! lifetime, which is not a contract worth having at this boundary.
//!
//! Phase 2 copies it as opaque pointer-sized words, because the typed struct — one correctly
//! typed `extern "C" fn` per hook — is generated code that arrives with Phase 3. The copy is
//! what the generated implementation will read through, so the ownership and lifetime story
//! does not change when the type does.

use std::ffi::c_void;
use std::sync::Arc;

/// An owned copy of the caller's hook table.
///
/// Cloning is cheap and shares the copy; each instance's hooks builder holds one.
#[derive(Clone)]
pub struct CapiHookTable {
    words: Arc<[*const c_void]>,
}

// The table is a block of plain function pointers with no interior mutability. The
// `ExperimentalExecutorRuntimeRef` it ends up behind is `Send + Sync`, so this has to be
// too; the pointers themselves are only ever read, and only from the thread servicing a
// host call.
unsafe impl Send for CapiHookTable {}
unsafe impl Sync for CapiHookTable {}

impl CapiHookTable {
    /// Copies `len` pointer-sized words out of `table`.
    ///
    /// # Safety
    ///
    /// `table` must point to at least `len` readable pointer-sized words.
    pub unsafe fn copy_from(table: *const c_void, len: usize) -> Self {
        let words: Arc<[*const c_void]> = if table.is_null() || len == 0 {
            Arc::from(Vec::new())
        } else {
            let src = unsafe { std::slice::from_raw_parts(table as *const *const c_void, len) };
            Arc::from(src.to_vec())
        };

        CapiHookTable { words }
    }

    /// Number of pointer-sized words copied.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// The `index`-th function pointer, or `None` if the table is shorter than that.
    ///
    /// Only used by the generated hooks implementation, which knows the real layout.
    pub fn word(&self, index: usize) -> Option<*const c_void> {
        self.words.get(index).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_the_caller_s_words() {
        // Real addresses rather than small integers, so the test does not depend on what a
        // fabricated pointer value is allowed to be.
        let targets = [10u64, 20u64, 30u64];
        let table: [*const c_void; 3] = [
            (&targets[0] as *const u64).cast(),
            (&targets[1] as *const u64).cast(),
            (&targets[2] as *const u64).cast(),
        ];
        let copied = unsafe { CapiHookTable::copy_from(table.as_ptr() as *const c_void, 3) };

        assert_eq!(copied.len(), 3);
        assert_eq!(copied.word(0), Some(table[0]));
        assert_eq!(copied.word(2), Some(table[2]));
        assert_eq!(copied.word(3), None);
    }

    #[test]
    fn tolerates_an_absent_table() {
        let empty = unsafe { CapiHookTable::copy_from(std::ptr::null(), 0) };
        assert!(empty.is_empty());
        assert_eq!(empty.word(0), None);

        let null_with_len = unsafe { CapiHookTable::copy_from(std::ptr::null(), 10) };
        assert!(null_with_len.is_empty());
    }
}
