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

pub const BREAKPOINT_VALUE_NO_BREAKPOINT: u64 = 0;
pub const BREAKPOINT_VALUE_EXECUTION_FAILED: u64 = 1;
pub const BREAKPOINT_VALUE_OUT_OF_GAS: u64 = 4;
pub const BREAKPOINT_VALUE_MEMORY_LIMIT: u64 = 5;

#[derive(Clone, Debug, MemoryUsage)]
struct BreakpointGlobalIndex(GlobalIndex);

impl BreakpointGlobalIndex {
    fn breakpoint_value(&self) -> GlobalIndex {
        self.0
    }
}

#[derive(Debug)]
pub struct Breakpoint {
    global_indexes: Mutex<Option<BreakpointGlobalIndex>>,
}

impl Breakpoint {
    pub fn new() -> Self {
        Self {
            global_indexes: Mutex::new(None),
        }
    }
}

unsafe impl Send for Breakpoint {}
unsafe impl Sync for Breakpoint {}

impl ModuleMiddleware for Breakpoint {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionBreakpoint {
            global_indexes: self.global_indexes.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_index = self.global_indexes.lock().unwrap();

        if global_index.is_some() {
            panic!("Breakpoint::transform_module_info: Attempting to use a `Breakpoint` middleware from multiple modules.");
        }

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

        *global_index = Some(BreakpointGlobalIndex(breakpoint_value_global_index));
    }
}

impl MemoryUsage for Breakpoint {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.global_indexes.size_of_val(tracker)
            - mem::size_of_val(&self.global_indexes)
    }
}

#[derive(Debug)]
struct FunctionBreakpoint {
    global_indexes: BreakpointGlobalIndex,
}

impl FunctionMiddleware for FunctionBreakpoint {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        let should_insert_breakpoint = match operator {
            Operator::Call { .. } | Operator::CallIndirect { .. } => true,
            _ => false,
        };

        if should_insert_breakpoint {
            state.extend(&[
                Operator::GlobalGet {
                    global_index: self.global_indexes.breakpoint_value().as_u32(),
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

pub fn set_breakpoint_value(instance: &Instance, value: u64) {
    instance
        .exports
        .get_global(BREAKPOINT_VALUE)
        .expect(format!("Can't get `{}` from Instance", BREAKPOINT_VALUE).as_str())
        .set(value.into())
        .expect(format!("Can't set `{}` in Instance", BREAKPOINT_VALUE).as_str())
}

pub fn get_breakpoint_value(instance: &Instance) -> u64 {
    instance
        .exports
        .get_global(BREAKPOINT_VALUE)
        .expect(format!("Can't get `{}` from Instance", BREAKPOINT_VALUE).as_str())
        .get()
        .try_into()
        .expect(format!("`{}` from Instance has wrong type", BREAKPOINT_VALUE).as_str())
}
