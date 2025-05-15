use crate::WasmerInstance;
use anyhow::anyhow;
use multiversx_chain_vm_executor::{
    BreakpointValue, ExecutorError, Instance, InstanceCallResult, InstanceLegacy,
};

use std::rc::Rc;

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
    InstanceCallResult::RuntimeError(anyhow!("wrapped instance error: {err}").into())
}

impl Instance for WasmerProdInstance {
    fn call(&self, func_name: &str) -> InstanceCallResult {
        if !self.inner_instance_ref.has_function(func_name) {
            return InstanceCallResult::FunctionNotFound;
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

                if breakpoint_value != BreakpointValue::None {
                    return InstanceCallResult::Breakpoint(breakpoint_value);
                }

                wrap_runtime_error(err)
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

    fn get_points_used(&self) -> Result<u64, ExecutorError> {
        self.inner_instance_ref
            .get_points_used()
            .map_err(|err| anyhow!("wrapped instance error: {err}").into())
    }

    fn reset(&self) -> Result<(), ExecutorError> {
        self.inner_instance_ref
            .reset()
            .map_err(|err| anyhow!("wrapped instance error: {err}").into())
    }

    fn cache(&self) -> Result<Vec<u8>, ExecutorError> {
        self.inner_instance_ref
            .cache()
            .map_err(|err| anyhow!("wrapped instance error: {err}").into())
    }
}
