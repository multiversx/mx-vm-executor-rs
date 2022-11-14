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

use crate::wasmer_breakpoint::{Breakpoint, BREAKPOINT_VALUE_MEMORY_LIMIT};

const OPCODE_CONTROL_MEMORY_GROW: &str = "opcode_control_memory_grow";
const OPCODE_CONTROL_MEMORY_GROW_CACHED: &str = "opcode_control_memory_grow_cached";

#[derive(Clone, Debug, MemoryUsage)]
struct OpcodeControlGlobalIndexes(GlobalIndex, GlobalIndex);

impl OpcodeControlGlobalIndexes {
    fn memory_grow(&self) -> GlobalIndex {
        self.0
    }

    fn memory_grow_cached(&self) -> GlobalIndex {
        self.1
    }
}

#[derive(Debug)]
pub(crate) struct OpcodeControl {
    max_memory_grow: usize,
    max_memory_grow_delta: usize,
    breakpoint: Arc<Breakpoint>,
    global_indexes: Mutex<Option<OpcodeControlGlobalIndexes>>,
}

impl OpcodeControl {
    pub(crate) fn new(
        max_memory_grow: usize,
        max_memory_grow_delta: usize,
        breakpoint: Arc<Breakpoint>,
    ) -> Self {
        Self {
            max_memory_grow,
            max_memory_grow_delta,
            breakpoint,
            global_indexes: Mutex::new(None),
        }
    }
}

unsafe impl Send for OpcodeControl {}
unsafe impl Sync for OpcodeControl {}

impl ModuleMiddleware for OpcodeControl {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionOpcodeControl {
            max_memory_grow: self.max_memory_grow,
            max_memory_grow_delta: self.max_memory_grow_delta,
            breakpoint: self.breakpoint.clone(),
            global_indexes: self.global_indexes.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_indexes = self.global_indexes.lock().unwrap();

        if global_indexes.is_some() {
            panic!("OpcodeControl::transform_module_info: Attempting to use a `OpcodeControl` middleware from multiple modules.");
        }

        let memory_grow_global_index = module_info
            .globals
            .push(GlobalType::new(Type::I64, Mutability::Var));

        module_info
            .global_initializers
            .push(GlobalInit::I64Const(0));

        module_info.exports.insert(
            OPCODE_CONTROL_MEMORY_GROW.to_string(),
            ExportIndex::Global(memory_grow_global_index),
        );

        let memory_grow_cached_global_index = module_info
            .globals
            .push(GlobalType::new(Type::I64, Mutability::Var));

        module_info
            .global_initializers
            .push(GlobalInit::I64Const(0));

        module_info.exports.insert(
            OPCODE_CONTROL_MEMORY_GROW_CACHED.to_string(),
            ExportIndex::Global(memory_grow_cached_global_index),
        );

        *global_indexes = Some(OpcodeControlGlobalIndexes(
            memory_grow_global_index,
            memory_grow_cached_global_index,
        ));
    }
}

impl MemoryUsage for OpcodeControl {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.global_indexes.size_of_val(tracker)
            - mem::size_of_val(&self.global_indexes)
    }
}

#[derive(Debug)]
struct FunctionOpcodeControl {
    max_memory_grow: usize,
    max_memory_grow_delta: usize,
    breakpoint: Arc<Breakpoint>,
    global_indexes: OpcodeControlGlobalIndexes,
}

impl FunctionMiddleware for FunctionOpcodeControl {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        match operator {
            Operator::MemoryGrow { .. } => {
                // Check if memory limit (memory_grow >= max_memory_grow)
                state.extend(&[
                    Operator::GlobalGet {
                        global_index: self.global_indexes.memory_grow().as_u32(),
                    },
                    Operator::I64Const {
                        value: self.max_memory_grow as i64,
                    },
                    Operator::I64GeU,
                ]);

                // Insert breakpoint BREAKPOINT_VALUE_MEMORY_LIMIT if memory_grow >= max_memory_grow
                state.extend(
                    self.breakpoint
                        .as_ref()
                        .insert_breakpoint(BREAKPOINT_VALUE_MEMORY_LIMIT)
                        .iter(),
                );

                // Increment the memory_grow counter
                state.extend(&[
                    Operator::GlobalGet {
                        global_index: self.global_indexes.memory_grow().as_u32(),
                    },
                    Operator::I64Const { value: 1 as i64 },
                    Operator::I64Add,
                    Operator::GlobalSet {
                        global_index: self.global_indexes.memory_grow().as_u32(),
                    },
                ]);

                // Cache the memory_grow counter
                state.extend(&[Operator::GlobalSet {
                    global_index: self.global_indexes.memory_grow_cached().as_u32(),
                }]);

                // Check if memory limit (memory_grow_cached > max_memory_grow_delta)
                state.extend(&[
                    Operator::GlobalGet {
                        global_index: self.global_indexes.memory_grow_cached().as_u32(),
                    },
                    Operator::I64Const {
                        value: self.max_memory_grow_delta as i64,
                    },
                    Operator::I64GtU,
                ]);

                // Insert breakpoint BREAKPOINT_VALUE_MEMORY_LIMIT if memory_grow_cached > max_memory_grow_delta
                state.extend(
                    self.breakpoint
                        .as_ref()
                        .insert_breakpoint(BREAKPOINT_VALUE_MEMORY_LIMIT)
                        .iter(),
                );

                // Retrieve cached memory_grow counter
                state.extend(&[Operator::GlobalGet {
                    global_index: self.global_indexes.memory_grow_cached().as_u32(),
                }]);
            }
            _ => {}
        }

        state.push_operator(operator);

        Ok(())
    }
}

#[allow(dead_code)]
pub(crate) fn reset_memory_grow(instance: &Instance) {
    instance
        .exports
        .get_global(OPCODE_CONTROL_MEMORY_GROW)
        .expect(format!("Can't get `{}` from Instance", OPCODE_CONTROL_MEMORY_GROW).as_str())
        .set(0.into())
        .expect(format!("Can't set `{}` in Instance", OPCODE_CONTROL_MEMORY_GROW).as_str())
}
