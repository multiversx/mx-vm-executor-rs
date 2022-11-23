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
#[allow(dead_code)]
pub(crate) const BREAKPOINT_VALUE_EXECUTION_FAILED: u64 = 1;
pub(crate) const BREAKPOINT_VALUE_OUT_OF_GAS: u64 = 4;
pub(crate) const BREAKPOINT_VALUE_MEMORY_LIMIT: u64 = 5;

#[derive(Clone, Debug, MemoryUsage)]
struct BreakpointsGlobalIndex(GlobalIndex);

impl BreakpointsGlobalIndex {
    fn breakpoint_value_global_index(&self) -> GlobalIndex {
        self.0
    }
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

    pub(crate) fn generate_breakpoint_condition<'b>(&self, value: u64) -> Vec<Operator<'b>> {
        vec![
            Operator::If {
                ty: WpTypeOrFuncType::Type(WpType::EmptyBlockType),
            },
            Operator::I64Const {
                value: value as i64,
            },
            Operator::GlobalSet {
                global_index: self.get_breakpoints_global_index().as_u32(),
            },
            Operator::End,
        ]
    }

    fn get_breakpoints_global_index(&self) -> GlobalIndex {
        self.global_index
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .breakpoint_value_global_index()
    }
}

unsafe impl Send for Breakpoints {}
unsafe impl Sync for Breakpoints {} 

impl MemoryUsage for Breakpoints {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.global_index.size_of_val(tracker)
            - mem::size_of_val(&self.global_index)
    }
}

impl ModuleMiddleware for Breakpoints {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionBreakpoint {
            global_index: self.global_index.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_index = self.global_index.lock().unwrap();

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

        *global_index = Some(BreakpointsGlobalIndex(breakpoint_value_global_index));
    }
}

#[derive(Debug)]
struct FunctionBreakpoint {
    global_index: BreakpointsGlobalIndex,
}

impl FunctionMiddleware for FunctionBreakpoint {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        if matches!(
            operator,
            Operator::Call { .. } | Operator::CallIndirect { .. }
        ) {
            state.extend(&[
                Operator::GlobalGet {
                    global_index: self.global_index.breakpoint_value_global_index().as_u32(),
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

        state.push_operator(operator);

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
