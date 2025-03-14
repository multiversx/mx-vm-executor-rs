use std::{mem, sync::Arc};

use loupe::{MemoryUsage, MemoryUsageTracker};
use wasmer::{
    wasmparser::Operator, FunctionMiddleware, LocalFunctionIndex, MiddlewareError,
    MiddlewareReaderState, ModuleMiddleware,
};
use wasmer_types::ModuleInfo;

use crate::wasmer_helpers::MiddlewareWithProtectedGlobals;

#[derive(Debug)]
pub(crate) struct ProtectedGlobals {
    protected_middlewares: Vec<Arc<dyn MiddlewareWithProtectedGlobals>>,
}

impl ProtectedGlobals {
    pub(crate) fn new(protected_middlewares: Vec<Arc<dyn MiddlewareWithProtectedGlobals>>) -> Self {
        Self {
            protected_middlewares,
        }
    }

    fn get_protected_globals(&self) -> Vec<u32> {
        let mut protected_globals = vec![];
        for middleware in &self.protected_middlewares {
            protected_globals.extend(middleware.protected_globals())
        }
        protected_globals
    }
}

unsafe impl Send for ProtectedGlobals {}
unsafe impl Sync for ProtectedGlobals {}

impl MemoryUsage for ProtectedGlobals {
    fn size_of_val(&self, _tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self)
    }
}

impl ModuleMiddleware for ProtectedGlobals {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionProtectedGlobals {
            protected_globals: self.get_protected_globals(),
        })
    }

    fn transform_module_info(&self, _module_info: &mut ModuleInfo) {}
}

#[derive(Debug)]
struct FunctionProtectedGlobals {
    protected_globals: Vec<u32>,
}

impl FunctionProtectedGlobals {
    fn check_protected_globals_invalid_access(
        &self,
        operator: &Operator,
    ) -> Result<(), MiddlewareError> {
        if let Operator::GlobalSet { global_index } = *operator {
            if self.protected_globals.contains(&global_index) {
                return Err(MiddlewareError::new(
                    "protected_globals_middleware",
                    "protected globals invalid access",
                ));
            }
        }

        Ok(())
    }
}

impl FunctionMiddleware for FunctionProtectedGlobals {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        self.check_protected_globals_invalid_access(&operator)?;

        state.push_operator(operator);

        Ok(())
    }
}
