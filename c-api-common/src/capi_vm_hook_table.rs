//! The Go VM hook function-pointer table, owned by Rust.
//!
//! The table is **copied** at executor creation rather than referenced, as the legacy API
//! also does. Holding the caller's pointer instead would make the executor's validity depend
//! on the caller keeping a Go-side struct alive and unmoved for the executor's whole
//! lifetime, which is not a contract worth having at this boundary.

use std::sync::Arc;

use crate::capi_vm_hook_pointers::vm_exec_vm_hook_c_func_pointers;

/// An owned copy of the caller's hook table.
///
/// Cloning is cheap and shares the copy; each instance's hooks builder holds one.
#[derive(Clone, Debug)]
pub struct CapiVMHookTable {
    func_pointers: Arc<vm_exec_vm_hook_c_func_pointers>,
}

// The table is a block of plain function pointers with no interior mutability, and is only
// ever read. The `ExperimentalExecutorRuntimeRef` it ends up behind is `Send + Sync`, so this
// has to be too.
unsafe impl Send for CapiVMHookTable {}
unsafe impl Sync for CapiVMHookTable {}

impl CapiVMHookTable {
    /// Copies the table the caller provided.
    ///
    /// # Safety
    ///
    /// `func_pointers` must be a valid, non-null pointer to a fully populated table. Every
    /// entry is called without a further null check, because a partially filled table is not
    /// something this API can detect: a null function pointer and a wrong one are
    /// indistinguishable here.
    pub unsafe fn copy_from(func_pointers: *const vm_exec_vm_hook_c_func_pointers) -> Self {
        CapiVMHookTable {
            func_pointers: Arc::new(unsafe { (*func_pointers).clone() }),
        }
    }

    pub fn func_pointers(&self) -> &vm_exec_vm_hook_c_func_pointers {
        &self.func_pointers
    }
}
