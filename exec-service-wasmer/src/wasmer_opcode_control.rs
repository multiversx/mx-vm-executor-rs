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
    wasmer_helpers::{create_global_index, MiddlewareWithProtectedGlobals},
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

    fn get_memory_grow_count_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .memory_grow_count_global_index
    }

    fn get_operand_backup_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .operand_backup_global_index
    }

    fn get_protected_globals(&self) -> Vec<u32> {
        let mut protected_globals = self.protected_globals();
        protected_globals.extend(self.breakpoints_middleware.protected_globals());
        protected_globals.extend(self.metering_middleware.protected_globals());
        protected_globals
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
            protected_globals: self.get_protected_globals(),
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

impl MiddlewareWithProtectedGlobals for OpcodeControl {
    fn protected_globals(&self) -> Vec<u32> {
        vec![
            self.get_memory_grow_count_global_index().as_u32(),
            self.get_operand_backup_global_index().as_u32(),
        ]
    }
}

#[derive(Debug)]
struct FunctionOpcodeControl {
    max_memory_grow: usize,
    max_memory_grow_delta: usize,
    breakpoints_middleware: Arc<Breakpoints>,
    protected_globals: Vec<u32>,
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

    fn check_protected_globals_invalid_access<'b>(
        &self,
        operator: &Operator<'b>,
    ) -> Result<(), MiddlewareError> {
        if let Operator::GlobalSet { global_index } = *operator {
            if self.protected_globals.contains(&global_index) {
                return Err(MiddlewareError::new(
                    "opcode_control_middleware",
                    "protected globals invalid access",
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
        // Check for protected globals invalid access
        self.check_protected_globals_invalid_access(&operator)?;

        if matches!(operator, Operator::MemoryGrow { .. }) {
            self.inject_memory_grow_check(state);
        }

        state.push_operator(operator);

        Ok(())
    }
}
