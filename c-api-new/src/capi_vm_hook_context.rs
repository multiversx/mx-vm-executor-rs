//! The hook context: what a VM hook is handed, and what makes it safe to hand over.
//!
//! Under Wasmer 6 the instance state is a borrow of the host-call frame — memory and globals
//! are only reachable through the `StoreMut` inside `FunctionEnvMut`, so the state is valid
//! for exactly one hook invocation and not a moment longer. In Rust the borrow checker
//! enforces that. Over C it has to be enforced dynamically, which is what this module does.
//!
//! The four guarantees that are lost at the boundary, and their replacements:
//!
//! 1. **Lifetime.** Each activation gets a fresh, non-zero call token. The context records
//!    it, and the pool records which token is currently innermost. A state function accepts
//!    a context only while the two agree, so a context used after its hook returned is
//!    rejected with `VM_EXEC2_STALE_CONTEXT` instead of dereferencing a dead borrow.
//!
//! 2. **Exclusivity.** Contexts are activated in a stack, one per nesting level, so a hook
//!    that re-enters the VM (`executeOnDestContext` running a contract from inside a hook)
//!    cannot clobber its caller's context. Only the innermost is usable, which matches the
//!    fact that an outer call's `StoreMut` borrow is not available while an inner call holds
//!    one.
//!
//! 3. **Thread affinity.** cgo keeps a goroutine on its OS thread for the duration of a
//!    call, so the hook itself is fine. A goroutine *spawned* inside a hook must never touch
//!    the context: the pool is not synchronised, and the token check is not a substitute for
//!    a lock. This is an invariant on the caller, stated here and in the generated header.
//!
//! 4. **No panics.** Handled by `capi_safe_unwind` on every entry point, not here.
//!
//! The contexts themselves are individually boxed and kept in a pool for the lifetime of the
//! instance, rather than living on the Rust stack. A stack-allocated context would leave the
//! Go side holding a pointer into a frame that has since been reused, so even reading the
//! token to reject it would be undefined behaviour. Pooling costs one allocation per nesting
//! level, reused across calls, and makes the rejection path well-defined.

use std::cell::RefCell;
use std::ffi::c_void;

use multiversx_chain_vm_executor::VMHooks;
use multiversx_chain_vm_executor_wasmer_experimental::{
    ExperimentalInstanceState, ExperimentalVMHooksBuilder,
};

use crate::capi_vm_hook_table::CapiVMHookTable;
use crate::capi_vm_hooks::CapiVMHooks;

/// The value of `vm_exec2_hook_result_t::early_exit` that means "the hook returned normally".
///
/// A dedicated sentinel is needed because `0` is a real early-exit code: the Rust VM uses
/// `ASYNC_CALL_EARLY_EXIT_CODE = 0` and the Go VM has the same async-call case, so "code 0"
/// and "no code" have to stay distinguishable.
pub const VM_EXEC2_NO_EARLY_EXIT: u64 = u64::MAX;

/// What every VM hook returns.
///
/// One 16-byte struct covers every hook shape — `void`, `i32`, `i64` — and is returned in
/// registers under both SysV AMD64 (RAX:RDX) and AAPCS64 (X0:X1), so the early-exit channel
/// costs no memory traffic. A cgo `//export` function may return a C struct by value, which
/// is what makes this shape available to the Go side.
///
/// The early-exit *message* does not travel in this struct. A pointer cannot: cgo forbids
/// returning an unpinned Go pointer, and a fixed buffer cannot work because the messages are
/// unbounded user text. The Go VM does not need it to travel, because the hook has already
/// recorded the message in its own output state by the time it returns.
#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct vm_exec2_hook_result_t {
    /// The hook's own return value, widened to 64 bits. Ignored for a `void` hook, and for
    /// any hook that reports an early exit.
    pub value: i64,

    /// The early-exit code, or `VM_EXEC2_NO_EARLY_EXIT`.
    pub early_exit: u64,
}

impl vm_exec2_hook_result_t {
    /// The hook returned normally, with the given value (`0` for a `void` hook).
    pub fn ok(value: i64) -> Self {
        vm_exec2_hook_result_t {
            value,
            early_exit: VM_EXEC2_NO_EARLY_EXIT,
        }
    }

    /// The hook requested an early exit with the given code.
    pub fn early_exit(code: u64) -> Self {
        vm_exec2_hook_result_t {
            value: 0,
            early_exit: code,
        }
    }

    /// The early-exit code, if there is one.
    pub fn early_exit_code(self) -> Option<u64> {
        if self.early_exit == VM_EXEC2_NO_EARLY_EXIT {
            None
        } else {
            Some(self.early_exit)
        }
    }
}

/// What a VM hook is handed as its first argument.
///
/// Only `go_vm_hooks` is part of the layout the Go side mirrors; the rest is private to
/// Rust, and the whole pointer is what travels back into the state functions.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct vm_exec2_hook_context_t {
    /// The Go VM hooks object for this instance. Read by the Go side, never by Rust.
    pub go_vm_hooks: *mut c_void,

    /// Erased `&mut ExperimentalInstanceState<'_>`; null while no call is active.
    state: *mut c_void,

    /// Erased `&RefCell<CapiVMHookContextPool>`, for the token check.
    pool: *const c_void,

    /// This activation's call token; `0` while no call is active.
    token: u64,
}

/// Lifetime-erased form of the instance state pointer kept in a context.
///
/// The real type is `&mut ExperimentalInstanceState<'s>` for the `'s` of the host-call frame.
/// That lifetime cannot be expressed across C, so it is erased here and enforced at runtime
/// by the call token.
type ErasedInstanceState = ExperimentalInstanceState<'static>;

impl vm_exec2_hook_context_t {
    fn inactive() -> Self {
        vm_exec2_hook_context_t {
            go_vm_hooks: std::ptr::null_mut(),
            state: std::ptr::null_mut(),
            pool: std::ptr::null(),
            token: 0,
        }
    }
}

/// Borrows the instance state out of a context, after checking that the context is live.
///
/// This is the one place the erased pointer is reconstituted, so it is the one place the
/// token check has to be right.
///
/// # Safety
///
/// `ctx` must either be null or point to a `vm_exec2_hook_context_t` that this crate
/// activated. Pooled contexts are never freed before their instance, so a context pointer
/// obtained from a hook always satisfies this, live or not.
pub(crate) unsafe fn with_instance_state<R>(
    ctx: *mut vm_exec2_hook_context_t,
    f: impl FnOnce(&mut &mut ExperimentalInstanceState<'static>) -> R,
) -> Result<R, ContextError> {
    if ctx.is_null() {
        return Err(ContextError::NullContext);
    }

    let ctx = unsafe { &*ctx };
    if ctx.token == 0 || ctx.state.is_null() || ctx.pool.is_null() {
        return Err(ContextError::Stale);
    }

    // Shared borrow only, and released before `f` runs, so a hook is free to re-enter the
    // VM and activate a nested context while this one is being read.
    let active_token = {
        let pool = unsafe { &*(ctx.pool as *const RefCell<CapiVMHookContextPool>) };
        pool.borrow().active_token
    };
    if ctx.token != active_token {
        return Err(ContextError::Stale);
    }

    let mut state: &mut ErasedInstanceState =
        unsafe { &mut *(ctx.state as *mut ErasedInstanceState) };
    Ok(f(&mut state))
}

/// Why a context could not be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextError {
    NullContext,
    Stale,
}

/// The per-instance stack of hook contexts.
#[derive(Default)]
pub struct CapiVMHookContextPool {
    /// One boxed context per nesting level.
    ///
    /// The `Box` is load-bearing and not the redundancy clippy takes it for: a
    /// `Vec<vm_exec2_hook_context_t>` would move its elements when it grows, invalidating
    /// context pointers that outer hook invocations are still holding. Boxing each context
    /// individually pins its address for the instance's lifetime, which is what lets a
    /// stale pointer be read and rejected instead of being undefined behaviour.
    #[allow(clippy::vec_box)]
    slots: Vec<Box<vm_exec2_hook_context_t>>,

    /// Nesting depth of the currently active call.
    depth: usize,

    /// Monotonic token source. Never reused, so a context from a finished call can never
    /// coincide with a later one.
    next_token: u64,

    /// Token of the innermost active call, or `0` when no call is active.
    active_token: u64,
}

/// Builds the VM hooks for one host call, and owns the context lifecycle.
pub struct CapiVMHooksBuilder {
    go_vm_hooks: *mut c_void,
    hook_table: CapiVMHookTable,
    pool: RefCell<CapiVMHookContextPool>,
}

impl CapiVMHooksBuilder {
    pub fn new(go_vm_hooks: *mut c_void, hook_table: CapiVMHookTable) -> Self {
        CapiVMHooksBuilder {
            go_vm_hooks,
            hook_table,
            pool: RefCell::new(CapiVMHookContextPool::default()),
        }
    }

    /// Activates a context around `instance_state` and runs `f` with it.
    ///
    /// The context is deactivated before this returns, whether `f` panics or not, so a
    /// pointer that escapes is guaranteed to fail the token check rather than reach a dead
    /// borrow.
    pub fn with_hook_context<R>(
        &self,
        instance_state: &mut ExperimentalInstanceState<'_>,
        f: impl FnOnce(*mut vm_exec2_hook_context_t) -> R,
    ) -> R {
        let state_ptr = (instance_state as *mut ExperimentalInstanceState<'_>).cast::<c_void>();

        // Safety: the pointer is derived from a live `&mut` that outlives this call, and the
        // context holding it is deactivated before we return.
        unsafe { self.with_hook_context_raw(state_ptr, f) }
    }

    /// [`Self::with_hook_context`] over an already-erased state pointer.
    ///
    /// This is the seam the generated hooks implementation and the boundary tests use: the
    /// typed entry point above is the only place that knows the real state type, and
    /// everything downstream of here deals in the erased pointer anyway.
    ///
    /// # Safety
    ///
    /// `state` must point to a live `ExperimentalInstanceState` for the whole of `f`, unless
    /// `f` never reaches the state through the context.
    pub unsafe fn with_hook_context_raw<R>(
        &self,
        state: *mut c_void,
        f: impl FnOnce(*mut vm_exec2_hook_context_t) -> R,
    ) -> R {
        let (ctx_ptr, parent_token) = {
            let mut pool = self.pool.borrow_mut();

            let depth = pool.depth;
            if depth == pool.slots.len() {
                pool.slots
                    .push(Box::new(vm_exec2_hook_context_t::inactive()));
            }

            pool.next_token += 1;
            let token = pool.next_token;
            let parent_token = pool.active_token;
            pool.active_token = token;
            pool.depth = depth + 1;

            let pool_ptr = &self.pool as *const RefCell<CapiVMHookContextPool> as *const c_void;

            let slot = &mut pool.slots[depth];
            slot.go_vm_hooks = self.go_vm_hooks;
            slot.state = state;
            slot.pool = pool_ptr;
            slot.token = token;

            (&mut **slot as *mut vm_exec2_hook_context_t, parent_token)
        };

        // The pool borrow is released across `f`, because the hook will call back into the
        // state functions, which borrow it again.
        let guard = DeactivateOnDrop {
            builder: self,
            ctx_ptr,
            parent_token,
        };
        let result = f(ctx_ptr);
        drop(guard);

        result
    }

    fn deactivate(&self, ctx_ptr: *mut vm_exec2_hook_context_t, parent_token: u64) {
        let mut pool = self.pool.borrow_mut();
        pool.depth = pool.depth.saturating_sub(1);
        pool.active_token = parent_token;

        // Clearing the slot is belt and braces: the token check alone already rejects it.
        // It matters because it makes a stale read fail on the cheap check rather than on
        // the pool lookup, and leaves no dangling state pointer behind in the pool.
        let ctx = unsafe { &mut *ctx_ptr };
        ctx.state = std::ptr::null_mut();
        ctx.token = 0;
    }
}

/// Runs the deactivation even if the hook body unwinds.
struct DeactivateOnDrop<'a> {
    builder: &'a CapiVMHooksBuilder,
    ctx_ptr: *mut vm_exec2_hook_context_t,
    parent_token: u64,
}

impl Drop for DeactivateOnDrop<'_> {
    fn drop(&mut self) {
        self.builder.deactivate(self.ctx_ptr, self.parent_token);
    }
}

impl ExperimentalVMHooksBuilder for CapiVMHooksBuilder {
    fn with_vm_hooks(
        &self,
        instance_state_ref: &mut ExperimentalInstanceState,
        f: &mut dyn FnMut(&mut dyn VMHooks),
    ) {
        self.with_hook_context(instance_state_ref, |ctx| {
            // The hooks borrow the context, so they live exactly as long as this activation
            // — which is also exactly as long as the instance state they reach through it.
            f(&mut CapiVMHooks::new(ctx, self.hook_table.func_pointers()));
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_result_distinguishes_code_zero_from_no_code() {
        assert_eq!(vm_exec2_hook_result_t::ok(7).early_exit_code(), None);
        assert_eq!(
            vm_exec2_hook_result_t::early_exit(0).early_exit_code(),
            Some(0)
        );
        assert_eq!(
            vm_exec2_hook_result_t::early_exit(4).early_exit_code(),
            Some(4)
        );
    }

    #[test]
    fn hook_result_fits_in_two_registers() {
        assert_eq!(std::mem::size_of::<vm_exec2_hook_result_t>(), 16);
        assert_eq!(std::mem::align_of::<vm_exec2_hook_result_t>(), 8);
    }

    #[test]
    fn null_context_is_rejected() {
        let result = unsafe { with_instance_state(std::ptr::null_mut(), |_| ()) };
        assert_eq!(result, Err(ContextError::NullContext));
    }

    #[test]
    fn inactive_context_is_rejected() {
        let mut ctx = vm_exec2_hook_context_t::inactive();
        let result = unsafe { with_instance_state(&mut ctx, |_| ()) };
        assert_eq!(result, Err(ContextError::Stale));
    }
}
