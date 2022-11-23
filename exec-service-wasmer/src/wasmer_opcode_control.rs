use std::{
    mem,
    sync::{Arc, Mutex},
};

use loupe::{MemoryUsage, MemoryUsageTracker};
use wasmer::{
    wasmparser::Operator, ExportIndex, FunctionMiddleware, GlobalInit, GlobalType, Instance,
    LocalFunctionIndex, MiddlewareError, MiddlewareReaderState, ModuleMiddleware, Mutability, Type,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

use crate::wasmer_breakpoints::{Breakpoints, BREAKPOINT_VALUE_MEMORY_LIMIT};

const OPCODE_CONTROL_MEMORY_GROW_COUNT: &str = "opcode_control_memory_grow_count";
const OPCODE_CONTROL_OPERAND_BACKUP: &str = "opcode_control_operand_backup";

#[derive(Clone, Debug, MemoryUsage)]
struct OpcodeControlGlobalIndexes(GlobalIndex, GlobalIndex);

impl OpcodeControlGlobalIndexes {
    fn memory_grow_count_global_index(&self) -> GlobalIndex {
        self.0
    }

    fn operand_backup_global_index(&self) -> GlobalIndex {
        self.1
    }
}

#[derive(Debug)]
pub(crate) struct OpcodeControl {
    max_memory_grow: usize,
    max_memory_grow_delta: usize,
    breakpoints_middleware: Arc<Breakpoints>,
    global_indexes: Mutex<Option<OpcodeControlGlobalIndexes>>,
}

impl OpcodeControl {
    pub(crate) fn new(
        max_memory_grow: usize,
        max_memory_grow_delta: usize,
        breakpoints_middleware: Arc<Breakpoints>,
    ) -> Self {
        Self {
            max_memory_grow,
            max_memory_grow_delta,
            breakpoints_middleware,
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
            global_indexes: self.global_indexes.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_indexes = self.global_indexes.lock().unwrap();

        let memory_grow_count_global_index = module_info
            .globals
            .push(GlobalType::new(Type::I64, Mutability::Var));

        module_info
            .global_initializers
            .push(GlobalInit::I64Const(0));

        module_info.exports.insert(
            OPCODE_CONTROL_MEMORY_GROW_COUNT.to_string(),
            ExportIndex::Global(memory_grow_count_global_index),
        );

        let operand_backup_global_index = module_info
            .globals
            .push(GlobalType::new(Type::I64, Mutability::Var));

        module_info
            .global_initializers
            .push(GlobalInit::I64Const(0));

        module_info.exports.insert(
            OPCODE_CONTROL_OPERAND_BACKUP.to_string(),
            ExportIndex::Global(operand_backup_global_index),
        );

        *global_indexes = Some(OpcodeControlGlobalIndexes(
            memory_grow_count_global_index,
            operand_backup_global_index,
        ));
    }
}

#[derive(Debug)]
struct FunctionOpcodeControl {
    max_memory_grow: usize,
    max_memory_grow_delta: usize,
    breakpoints_middleware: Arc<Breakpoints>,
    global_indexes: OpcodeControlGlobalIndexes,
}

impl FunctionMiddleware for FunctionOpcodeControl {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        if let Operator::MemoryGrow { .. } = operator {
            // Before attempting anything with memory.grow, the current memory.grow
            // count is checked against the self.max_memory_grow limit.
            state.extend(&[
                Operator::GlobalGet {
                    global_index: self
                        .global_indexes
                        .memory_grow_count_global_index()
                        .as_u32(),
                },
                Operator::I64Const {
                    value: self.max_memory_grow as i64,
                },
                Operator::I64GeU,
            ]);

            // Insert breakpoint BREAKPOINT_VALUE_MEMORY_LIMIT.
            state.extend(
                self.breakpoints_middleware
                    .as_ref()
                    .generate_breakpoint_condition(BREAKPOINT_VALUE_MEMORY_LIMIT)
                    .iter(),
            );

            // Increment memory.grow counter.
            state.extend(&[
                Operator::GlobalGet {
                    global_index: self
                        .global_indexes
                        .memory_grow_count_global_index()
                        .as_u32(),
                },
                Operator::I64Const { value: 1 },
                Operator::I64Add,
                Operator::GlobalSet {
                    global_index: self
                        .global_indexes
                        .memory_grow_count_global_index()
                        .as_u32(),
                },
            ]);

            // Backup the top of the stack (the parameter for memory.grow) in order to
            // duplicate it: once for the comparison against max_memory_grow_delta and
            // again for memory.grow itself, assuming the comparison passes.
            state.extend(&[Operator::GlobalSet {
                global_index: self.global_indexes.operand_backup_global_index().as_u32(),
            }]);

            // Set up the comparison against max_memory_grow_delta.
            state.extend(&[
                Operator::GlobalGet {
                    global_index: self.global_indexes.operand_backup_global_index().as_u32(),
                },
                Operator::I64Const {
                    value: self.max_memory_grow_delta as i64,
                },
                Operator::I64GtU,
            ]);

            // Insert breakpoint BREAKPOINT_VALUE_MEMORY_LIMIT.
            state.extend(
                self.breakpoints_middleware
                    .as_ref()
                    .generate_breakpoint_condition(BREAKPOINT_VALUE_MEMORY_LIMIT)
                    .iter(),
            );

            // Bring back the backed-up operand for memory.grow.
            state.extend(&[Operator::GlobalGet {
                global_index: self.global_indexes.operand_backup_global_index().as_u32(),
            }]);
        }

        state.push_operator(operator);

        Ok(())
    }
}

#[allow(dead_code)]
pub(crate) fn reset_memory_grow_count(instance: &Instance) {
    instance
        .exports
        .get_global(OPCODE_CONTROL_MEMORY_GROW_COUNT)
        .unwrap_or_else(|_| {
            panic!(
                "Can't get `{}` from Instance",
                OPCODE_CONTROL_MEMORY_GROW_COUNT
            )
        })
        .set(0.into())
        .unwrap_or_else(|_| {
            panic!(
                "Can't set `{}` in Instance",
                OPCODE_CONTROL_MEMORY_GROW_COUNT
            )
        })
}
