use std::{
    mem,
    sync::{Arc, Mutex},
};

use loupe::{MemoryUsage, MemoryUsageTracker};
use wasmer::{
    wasmparser::Operator, FunctionMiddleware, LocalFunctionIndex, MiddlewareError,
    MiddlewareReaderState, ModuleMiddleware,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

use crate::{
    wasmer_breakpoints::{Breakpoints, BREAKPOINT_VALUE_MEMORY_LIMIT},
    wasmer_helpers::create_global_index,
    wasmer_metering::Metering,
};

const OPCODE_CONTROL_MEMORY_GROW_COUNT: &str = "opcode_control_memory_grow_count";
const OPCODE_CONTROL_OPERAND_BACKUP: &str = "opcode_control_operand_backup";

#[derive(Clone, Debug, MemoryUsage)]
struct OpcodeControlGlobalIndexes {
    memory_grow_count_global_index: GlobalIndex,
    operand_backup_global_index: GlobalIndex,
}

#[derive(Debug)]
pub(crate) struct OpcodeControl {
    max_memory_grow: usize,
    max_memory_grow_delta: usize,
    breakpoints_middleware: Arc<Breakpoints>,
    metering_middleware: Arc<Metering>,
    global_indexes: Mutex<Option<OpcodeControlGlobalIndexes>>,
}

impl OpcodeControl {
    pub(crate) fn new(
        max_memory_grow: usize,
        max_memory_grow_delta: usize,
        breakpoints_middleware: Arc<Breakpoints>,
        metering_middleware: Arc<Metering>,
    ) -> Self {
        Self {
            max_memory_grow,
            max_memory_grow_delta,
            breakpoints_middleware,
            metering_middleware,
            global_indexes: Mutex::new(None),
        }
    }
}

unsafe impl Send for OpcodeControl {}
unsafe impl Sync for OpcodeControl {}

impl MemoryUsage for OpcodeControl {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.global_indexes.size_of_val(tracker)
            - mem::size_of_val(&self.global_indexes)
    }
}

impl ModuleMiddleware for OpcodeControl {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionOpcodeControl {
            max_memory_grow: self.max_memory_grow,
            max_memory_grow_delta: self.max_memory_grow_delta,
            breakpoints_middleware: self.breakpoints_middleware.clone(),
            metering_middleware: self.metering_middleware.clone(),
            global_indexes: self.global_indexes.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_indexes = self.global_indexes.lock().unwrap();

        *global_indexes = Some(OpcodeControlGlobalIndexes {
            memory_grow_count_global_index: create_global_index(
                module_info,
                OPCODE_CONTROL_MEMORY_GROW_COUNT,
                0,
            ),
            operand_backup_global_index: create_global_index(
                module_info,
                OPCODE_CONTROL_OPERAND_BACKUP,
                0,
            ),
        });
    }
}

#[derive(Debug)]
struct FunctionOpcodeControl {
    max_memory_grow: usize,
    max_memory_grow_delta: usize,
    breakpoints_middleware: Arc<Breakpoints>,
    metering_middleware: Arc<Metering>,
    global_indexes: OpcodeControlGlobalIndexes,
}

impl FunctionOpcodeControl {
    fn inject_memory_grow_limit_check<'b>(&self, state: &mut MiddlewareReaderState<'b>) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_indexes.memory_grow_count_global_index.as_u32(),
            },
            Operator::I64Const {
                value: self.max_memory_grow as i64,
            },
            Operator::I64GeU,
        ]);
        self.breakpoints_middleware
            .inject_breakpoint_condition(state, BREAKPOINT_VALUE_MEMORY_LIMIT);
    }

    fn inject_memory_grow_count_increment<'b>(&self, state: &mut MiddlewareReaderState<'b>) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_indexes.memory_grow_count_global_index.as_u32(),
            },
            Operator::I64Const { value: 1 },
            Operator::I64Add,
            Operator::GlobalSet {
                global_index: self.global_indexes.memory_grow_count_global_index.as_u32(),
            },
        ]);
    }

    fn inject_memory_grow_delta_limit_check<'b>(&self, state: &mut MiddlewareReaderState<'b>) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_indexes.operand_backup_global_index.as_u32(),
            },
            Operator::I64Const {
                value: self.max_memory_grow_delta as i64,
            },
            Operator::I64GtU,
        ]);

        self.breakpoints_middleware
            .inject_breakpoint_condition(state, BREAKPOINT_VALUE_MEMORY_LIMIT);
    }

    fn inject_memory_grow_check<'b>(&self, state: &mut MiddlewareReaderState<'b>) {
        self.inject_memory_grow_limit_check(state);
        self.inject_memory_grow_count_increment(state);

        // Backup the top of the stack (the parameter for memory.grow) in order to
        // duplicate it: once for the comparison against max_memory_grow_delta and
        // again for memory.grow itself, assuming the comparison passes.
        state.extend(&[Operator::GlobalSet {
            global_index: self.global_indexes.operand_backup_global_index.as_u32(),
        }]);

        self.inject_memory_grow_delta_limit_check(state);

        // Bring back the backed-up operand for memory.grow.
        state.extend(&[Operator::GlobalGet {
            global_index: self.global_indexes.operand_backup_global_index.as_u32(),
        }]);
    }

    fn check_invalid_global_set<'b>(&self, operator: &Operator<'b>) -> Result<(), MiddlewareError> {
        if let Operator::GlobalSet { global_index } = *operator {
            if global_index
                == self
                    .breakpoints_middleware
                    .get_breakpoint_value_global_index()
                    .as_u32()
            {
                return Err(MiddlewareError::new(
                    "breakpoints_middleware",
                    "invalid global set",
                ));
            }

            if global_index
                == self
                    .metering_middleware
                    .get_points_limit_global_index()
                    .as_u32()
                || global_index
                    == self
                        .metering_middleware
                        .get_points_used_global_index()
                        .as_u32()
            {
                return Err(MiddlewareError::new(
                    "metering_middleware",
                    "invalid global set",
                ));
            }

            if global_index == self.global_indexes.memory_grow_count_global_index.as_u32()
                || global_index == self.global_indexes.operand_backup_global_index.as_u32()
            {
                return Err(MiddlewareError::new(
                    "opcode_control_middleware",
                    "invalid global set",
                ));
            }
        }

        Ok(())
    }
}

impl FunctionMiddleware for FunctionOpcodeControl {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        // Check for invalid access of middlewares globals
        self.check_invalid_global_set(&operator)?;

        if matches!(operator, Operator::MemoryGrow { .. }) {
            self.inject_memory_grow_check(state);
        }

        state.push_operator(operator);

        Ok(())
    }
}
