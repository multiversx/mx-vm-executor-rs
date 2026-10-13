use crate::executor_interface::{
    BreakpointValue, BreakpointValueLegacy, ExecutorError, Instance, InstanceCallResult,
    InstanceLegacy,
};
use crate::{WasmerInstance, wasmer_metering::set_points_limit};

use std::rc::Rc;

use super::WasmerExecutorError;

#[derive(Clone)]
pub struct WasmerProdInstance {
    inner_instance_ref: Rc<WasmerInstance>,
}

impl WasmerProdInstance {
    pub fn new(inner_instance_ref: Rc<WasmerInstance>) -> Self {
        WasmerProdInstance { inner_instance_ref }
    }
}

fn wrap_runtime_error(err: String) -> InstanceCallResult {
    InstanceCallResult::RuntimeError(WasmerExecutorError::InstanceCall(err).into())
}

impl Instance for WasmerProdInstance {
    fn call(&mut self, func_name: &str, points_limit: u64) -> InstanceCallResult {
        if !self.inner_instance_ref.has_function(func_name) {
            return InstanceCallResult::FunctionNotFound;
        }

        if let Err(err) = set_points_limit(&self.inner_instance_ref.wasmer_instance, points_limit) {
            return InstanceCallResult::RuntimeError(
                WasmerExecutorError::SetPointsLimit(err).into(),
            );
        }

        // Clear the breakpoint global, so the instance is armed for this call rather than
        // still halted from the last one.
        //
        // Under the legacy interface this was the *host's* job — the Go VM did it in
        // `runtimeContext.UseWarmInstance`, and the Rust VM never reused an instance at all.
        // The new interface does not expose the global, deliberately: it is 2.2's private
        // mechanism for stopping a contract, and Wasmer 7 has nothing like it. So the only
        // side that can still do this is the executor, and not doing it would make an
        // instance single-use after any early exit — a divergence from Wasmer 7, which has no
        // such state to leave behind.
        if let Err(err) = self
            .inner_instance_ref
            .set_breakpoint_value(BreakpointValueLegacy::None)
        {
            return wrap_runtime_error(err);
        }

        let result = self.inner_instance_ref.call(func_name);

        match result {
            Ok(()) => InstanceCallResult::Ok,
            Err(err) => {
                if let Some(early_exit) = self.inner_instance_ref.take_early_exit() {
                    return InstanceCallResult::VMHooksEarlyExit(early_exit);
                }

                let breakpoint_value = match self.inner_instance_ref.get_breakpoint_value() {
                    Ok(breakpoint_value) => breakpoint_value,
                    Err(err) => {
                        return wrap_runtime_error(err);
                    }
                };

                match breakpoint_value {
                    BreakpointValueLegacy::OutOfGas => {
                        InstanceCallResult::Breakpoint(BreakpointValue::OutOfGas)
                    }
                    BreakpointValueLegacy::MemoryLimit => {
                        InstanceCallResult::Breakpoint(BreakpointValue::MemoryLimit)
                    }
                    _ => wrap_runtime_error(err),
                }
            }
        }
    }

    fn check_signatures(&self) -> bool {
        self.inner_instance_ref.check_signatures()
    }

    fn has_function(&self, func_name: &str) -> bool {
        self.inner_instance_ref.has_function(func_name)
    }

    fn get_exported_function_names(&self) -> Vec<String> {
        self.inner_instance_ref.get_exported_function_names()
    }

    fn get_imported_function_names(&self) -> Vec<String> {
        self.inner_instance_ref.get_imported_function_names()
    }

    fn get_points_used(&mut self) -> Result<u64, ExecutorError> {
        self.inner_instance_ref
            .get_points_used()
            .map_err(|err| WasmerExecutorError::WrappedInstance(err).into())
    }

    fn reset(&mut self) -> Result<(), ExecutorError> {
        self.inner_instance_ref
            .reset()
            .map_err(|err| WasmerExecutorError::WrappedInstance(err).into())
    }

    fn cache(&self) -> Result<Vec<u8>, ExecutorError> {
        self.inner_instance_ref
            .cache()
            .map_err(|err| WasmerExecutorError::WrappedInstance(err).into())
    }
}
