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

    fn get_points_used(&mut self) -> Result<u64, ExecutorError> {
        self.inner_instance_ref
            .get_points_used()
            .map_err(|err| WasmerExecutorError::WrappedInstance(err).into())
    }

    fn reset(&self) -> Result<(), ExecutorError> {
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
