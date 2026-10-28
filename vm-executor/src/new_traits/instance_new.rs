use std::fmt;

use crate::{BreakpointValue, ExecutorError, VMHooksEarlyExit};

pub enum InstanceCallResult {
    Ok,
    FunctionNotFound,
    RuntimeError(ExecutorError),
    VMHooksEarlyExit(VMHooksEarlyExit),
    Breakpoint(BreakpointValue),

    /// The host asked for the call to stop, through an [`InstanceStopHandle`].
    ///
    /// Its own kind rather than a `BreakpointValue`, because it is not one: that enum covers
    /// the interruptions the executor's *own middlewares* raise, and this one comes from
    /// outside the executor entirely.
    Interrupted,
}

/// Stops a call that is already running.
///
/// Separate from [`Instance`] on purpose. The only moment a stop is useful is while `call` is
/// in flight, and `call` takes `&mut self` — so a method on the instance could not be reached
/// precisely when it is wanted, and reaching it anyway would alias that borrow. A handle is
/// taken *before* the call and used from another thread during it, which is the same shape
/// Wasmer 7 settled on for `Store::interrupter`.
///
/// It is best effort, and silent about whether it worked, for two reasons that no
/// implementation can do anything about: a contract already past its last checkpoint may
/// finish first, and neither backend can interrupt a contract that is inside a VM hook — the
/// stop is noticed when control returns to wasm. A caller that must bound the wait needs its
/// own timeout regardless.
pub trait InstanceStopHandle: Send + Sync + fmt::Debug {
    /// Asks the instance to stop at the next opportunity.
    ///
    /// Safe to call at any time, from any thread, including when no call is running or when
    /// the call has already finished: both are no-ops.
    fn request_stop(&self);
}

/// The new instance trait, only used for configuring & calling the wasmer instance.
pub trait Instance {
    /// Calls an exported function of a WebAssembly instance by `name`.
    fn call(&mut self, func_name: &str, points_limit: u64) -> InstanceCallResult;

    /// Checks that all public module functions (SC endpoints) have no arguments or results.
    fn check_signatures(&self) -> bool;

    /// Checks whether SC has an endpoint with given name.
    fn has_function(&self, func_name: &str) -> bool;

    /// Required to be able to extract all SC endpoint names.
    fn get_exported_function_names(&self) -> Vec<String>;

    /// Returns the names of all the host functions (VM hooks) the contract imports.
    ///
    /// Needed to validate a contract against the set of hooks it is allowed to use. All the
    /// names are returned at once, rather than offering a per-name query, because the
    /// callers check the module against whole sets of names.
    fn get_imported_function_names(&self) -> Vec<String>;

    /// Returns the number of points(gas) used by the given instance.
    fn get_points_used(&mut self) -> Result<u64, ExecutorError>;

    /// Overwrites the number of points (gas) used by the given instance.
    ///
    /// The counterpart of [`Instance::get_points_used`], and here for the same reason: the
    /// points counter belongs to the instance and outlives any single call, so a host that
    /// charges gas between calls has nowhere else to put it.
    ///
    /// [`InstanceState::set_points_used`] is the same operation seen from inside a host call.
    /// Both exist because both moments are real: a hook charges gas while the contract is
    /// running, and a host charges gas after a call has returned but before the next one on
    /// the same instance — settling an async step, refunding what a nested execution did not
    /// spend. The Go VM does both, and the counter it adjusts is the one the metering
    /// middleware compares against the limit on the next call.
    ///
    /// [`InstanceState::set_points_used`]: crate::InstanceState::set_points_used
    fn set_points_used(&mut self, points: u64) -> Result<(), ExecutorError>;

    /// Returns the instance to the state it was in immediately after creation, so that it can
    /// serve another execution.
    ///
    /// Memory contents and length, every mutable global, and anything else a call can leave
    /// behind must be restored. What must *not* change is the instance's identity: the host
    /// holds this instance across the reset, so a backend has to rebuild in place rather than
    /// hand back a new object.
    ///
    /// Takes `&mut self` because that is what it is: the whole point is to mutate the
    /// instance. A backend may need to replace its internals wholesale — see
    /// `ExperimentalInstance::reset`, which rebuilds the store — and `&self` would force it to
    /// reach for interior mutability to express something that is not shared at all.
    ///
    /// A backend that cannot offer this returns an error, and the host is expected to discard
    /// the instance and build a fresh one.
    fn reset(&mut self) -> Result<(), ExecutorError>;

    /// Caches an instance.
    fn cache(&self) -> Result<Vec<u8>, ExecutorError>;

    /// Returns a handle that can stop a call on this instance from another thread.
    ///
    /// Taken before the call it is meant to stop; see [`InstanceStopHandle`] for why it is a
    /// handle rather than a method here.
    fn stop_handle(&self) -> Box<dyn InstanceStopHandle>;
}
