use crate::WasmerInstance;
use log::trace;
use multiversx_chain_vm_executor::{
    CompilationOptions, Executor, ExecutorError, Instance, OpcodeCost, VMHooksBuilder,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct WasmerExecutorData {
    pub vm_hooks_builder: Rc<dyn VMHooksBuilder>,
    pub opcode_cost: Arc<Mutex<OpcodeCost>>,
}

impl WasmerExecutorData {
    pub fn new(vm_hooks_builder: Rc<dyn VMHooksBuilder>) -> Self {
        Self {
            vm_hooks_builder,
            opcode_cost: Arc::new(Mutex::new(OpcodeCost::default())),
        }
    }

    fn set_opcode_cost(&mut self, opcode_cost: &OpcodeCost) -> Result<(), ExecutorError> {
        self.opcode_cost.lock().unwrap().clone_from(opcode_cost);
        Ok(())
    }

    pub(crate) fn get_opcode_cost(&self) -> Arc<Mutex<OpcodeCost>> {
        self.opcode_cost.clone()
    }
}

pub struct WasmerExecutor {
    data: Rc<RefCell<WasmerExecutorData>>,
}

impl WasmerExecutor {
    pub fn new(vm_hooks_builder: Rc<dyn VMHooksBuilder>) -> Self {
        Self {
            data: Rc::new(RefCell::new(WasmerExecutorData::new(vm_hooks_builder))),
        }
    }
}

impl Executor for WasmerExecutor {
    fn set_opcode_cost(&mut self, opcode_cost: &OpcodeCost) -> Result<(), ExecutorError> {
        trace!("Setting opcode cost...");
        self.data.borrow_mut().set_opcode_cost(opcode_cost)
    }

    fn new_instance(
        &self,
        wasm_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        let instance =
            WasmerInstance::try_new_instance(self.data.clone(), wasm_bytes, compilation_options)?;
        Ok(Box::new(instance))
    }

    fn new_instance_from_cache(
        &self,
        cache_bytes: &[u8],
        compilation_options: &CompilationOptions,
    ) -> Result<Box<dyn Instance>, ExecutorError> {
        let instance = WasmerInstance::try_new_instance_from_cache(
            self.data.clone(),
            cache_bytes,
            compilation_options,
        )?;
        Ok(Box::new(instance))
    }
}
