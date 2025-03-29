use std::sync::Mutex;

use wasmer::wasmparser::Operator;
use wasmer::{
    AsStoreMut, FunctionMiddleware, Instance, LocalFunctionIndex, MiddlewareError,
    MiddlewareReaderState, ModuleMiddleware,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

use crate::we_helpers::{
    create_global_index, get_global_value_u64, is_control_flow_operator, set_global_value_u64,
};

const BREAKPOINT_VALUE: &str = "breakpoint_value";

pub(crate) const BREAKPOINT_VALUE_NO_BREAKPOINT: u64 = 0;
pub(crate) const BREAKPOINT_VALUE_OUT_OF_GAS: u64 = 4;
pub(crate) const BREAKPOINT_VALUE_MEMORY_LIMIT: u64 = 5;

#[derive(Clone, Debug)]
struct BreakpointsGlobalIndex {
    breakpoint_value_global_index: GlobalIndex,
}

#[derive(Debug)]
pub(crate) struct Breakpoints {
    global_index: Mutex<Option<BreakpointsGlobalIndex>>,
}

impl Breakpoints {
    pub(crate) fn new() -> Self {
        Self {
            global_index: Mutex::new(None),
        }
    }

    pub(crate) fn inject_breakpoint_condition(
        &self,
        state: &mut MiddlewareReaderState,
        breakpoint_value: u64,
    ) {
        state.extend(&[
            Operator::If {
                blockty: wasmer::wasmparser::BlockType::Empty,
            },
            Operator::I64Const {
                value: breakpoint_value as i64,
            },
            Operator::GlobalSet {
                global_index: self.get_breakpoint_value_global_index().as_u32(),
            },
            Operator::End,
        ]);
    }

    pub fn get_breakpoint_value_global_index(&self) -> GlobalIndex {
        self.global_index
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .breakpoint_value_global_index
    }
}

unsafe impl Send for Breakpoints {}
unsafe impl Sync for Breakpoints {}

impl ModuleMiddleware for Breakpoints {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionBreakpoints {
            global_index: self.global_index.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) -> Result<(), MiddlewareError> {
        let mut global_index = self.global_index.lock().unwrap();

        *global_index = Some(BreakpointsGlobalIndex {
            breakpoint_value_global_index: create_global_index(
                module_info,
                BREAKPOINT_VALUE,
                BREAKPOINT_VALUE_NO_BREAKPOINT as i64,
            ),
        });

        Ok(())
    }
}

#[derive(Debug)]
struct FunctionBreakpoints {
    global_index: BreakpointsGlobalIndex,
}

impl FunctionBreakpoints {
    fn inject_breakpoint_condition_check(&self, state: &mut MiddlewareReaderState) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_index.breakpoint_value_global_index.as_u32(),
            },
            Operator::I64Const {
                value: BREAKPOINT_VALUE_NO_BREAKPOINT as i64,
            },
            Operator::I64Ne,
            Operator::If {
                blockty: wasmer::wasmparser::BlockType::Empty,
            },
            Operator::Unreachable,
            Operator::End,
        ]);
    }
}

impl FunctionMiddleware for FunctionBreakpoints {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        let must_add_breakpoint = is_control_flow_operator(&operator);

        state.push_operator(operator);

        if must_add_breakpoint {
            self.inject_breakpoint_condition_check(state)
        }

        Ok(())
    }
}

pub(crate) fn set_breakpoint_value(
    instance: &Instance,
    store: &mut impl AsStoreMut,
    value: u64,
) -> Result<(), String> {
    set_global_value_u64(instance, store, BREAKPOINT_VALUE, value)
}

pub(crate) fn get_breakpoint_value(
    instance: &Instance,
    store: &mut impl AsStoreMut,
) -> Result<u64, String> {
    get_global_value_u64(instance, store, BREAKPOINT_VALUE)
}
