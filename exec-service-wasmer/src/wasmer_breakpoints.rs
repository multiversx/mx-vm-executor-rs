use std::mem;
use std::sync::Mutex;

use loupe::{MemoryUsage, MemoryUsageTracker};
use wasmer::wasmparser::{Operator, Type as WpType, TypeOrFuncType as WpTypeOrFuncType};
use wasmer::{
    ExportIndex, FunctionMiddleware, GlobalInit, GlobalType, Instance, LocalFunctionIndex,
    MiddlewareError, MiddlewareReaderState, ModuleMiddleware, Mutability, Type,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

const BREAKPOINT_VALUE: &str = "breakpoint_value";

pub(crate) const BREAKPOINT_VALUE_NO_BREAKPOINT: u64 = 0;
pub(crate) const BREAKPOINT_VALUE_OUT_OF_GAS: u64 = 4;
pub(crate) const BREAKPOINT_VALUE_MEMORY_LIMIT: u64 = 5;

#[derive(Clone, Debug, MemoryUsage)]
struct BreakpointsGlobalIndexes {
    breakpoint_value_global_index: GlobalIndex,
}

#[derive(Debug)]
pub(crate) struct Breakpoints {
    global_indexes: Mutex<Option<BreakpointsGlobalIndexes>>,
}

impl Breakpoints {
    pub(crate) fn new() -> Self {
        Self {
            global_indexes: Mutex::new(None),
        }
    }

    pub(crate) fn inject_breakpoint_condition<'b>(
        &self,
        state: &mut MiddlewareReaderState<'b>,
        breakpoint_value: u64,
    ) {
        state.extend(&[
            Operator::If {
                ty: WpTypeOrFuncType::Type(WpType::EmptyBlockType),
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

    fn get_breakpoint_value_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .breakpoint_value_global_index
    }
}

unsafe impl Send for Breakpoints {}
unsafe impl Sync for Breakpoints {}

impl MemoryUsage for Breakpoints {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.global_indexes.size_of_val(tracker)
            - mem::size_of_val(&self.global_indexes)
    }
}

impl ModuleMiddleware for Breakpoints {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionBreakpoints {
            global_indexes: self.global_indexes.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_indexes = self.global_indexes.lock().unwrap();

        let breakpoint_value_global_index = module_info
            .globals
            .push(GlobalType::new(Type::I64, Mutability::Var));

        module_info
            .global_initializers
            .push(GlobalInit::I64Const(BREAKPOINT_VALUE_NO_BREAKPOINT as i64));

        module_info.exports.insert(
            BREAKPOINT_VALUE.to_string(),
            ExportIndex::Global(breakpoint_value_global_index),
        );

        *global_indexes = Some(BreakpointsGlobalIndexes {
            breakpoint_value_global_index,
        });
    }
}

#[derive(Debug)]
struct FunctionBreakpoints {
    global_indexes: BreakpointsGlobalIndexes,
}

impl FunctionBreakpoints {
    fn inject_breakpoint_condition_check<'b>(&self, state: &mut MiddlewareReaderState<'b>) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_indexes.breakpoint_value_global_index.as_u32(),
            },
            Operator::I64Const {
                value: BREAKPOINT_VALUE_NO_BREAKPOINT as i64,
            },
            Operator::I64Ne,
            Operator::If {
                ty: WpTypeOrFuncType::Type(WpType::EmptyBlockType),
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
        let must_add_breakpoint = if matches!(
            operator,
            Operator::Call { .. } | Operator::CallIndirect { .. }
        ) {
            true
        } else {
            false
        };

        state.push_operator(operator);

        if must_add_breakpoint {
            self.inject_breakpoint_condition_check(state)
        }

        Ok(())
    }
}

pub(crate) fn set_breakpoint_value(instance: &Instance, value: u64) {
    instance
        .exports
        .get_global(BREAKPOINT_VALUE)
        .unwrap_or_else(|_| panic!("Can't get `{}` from Instance", BREAKPOINT_VALUE))
        .set(value.into())
        .unwrap_or_else(|_| panic!("Can't set `{}` in Instance", BREAKPOINT_VALUE))
}

pub(crate) fn get_breakpoint_value(instance: &Instance) -> u64 {
    instance
        .exports
        .get_global(BREAKPOINT_VALUE)
        .unwrap_or_else(|_| panic!("Can't get `{}` from Instance", BREAKPOINT_VALUE))
        .get()
        .try_into()
        .unwrap_or_else(|_| panic!("`{}` from Instance has wrong type", BREAKPOINT_VALUE))
}
