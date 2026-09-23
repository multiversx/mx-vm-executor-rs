#![allow(unused)] // TODO: until we activate the local count mechanism

use super::{
    BREAKPOINT_VALUE_OUT_OF_GAS, Breakpoints, Cost, MiddlewareWithProtectedGlobals, get_opcode_cost,
};
use crate::we_helpers::{
    create_global_index, get_global_value_u64, is_control_flow_operator, set_global_value_u64,
};
use multiversx_chain_vm_executor::{ExecutorError, OpcodeConfig, OpcodeCost};
use std::mem;
use std::sync::{Arc, Mutex};
use wasmer::sys::{FunctionMiddleware, MiddlewareReaderState, ModuleMiddleware};
use wasmer::wasmparser::Operator;
use wasmer::{AsStoreMut, Instance, LocalFunctionIndex};
use wasmer_types::{GlobalIndex, MiddlewareError, ModuleInfo};

const METERING_POINTS_LIMIT: &str = "metering_points_limit";
const METERING_POINTS_USED: &str = "metering_points_used";
const METERING_BULK_MEMORY_SIZE_OPERAND_BACKUP: &str = "metering_bulk_memory_size_operand_backup";
const MAX_LOCAL_COUNT: u32 = 4000;
const POINTS_LIMIT_INIT: i64 = 0;

#[derive(Clone, Debug)]
struct MeteringGlobalIndexes {
    points_limit_global_index: GlobalIndex,
    points_used_global_index: GlobalIndex,
    bulk_memory_size_operand_backup_global_index: GlobalIndex,
}

#[derive(Debug)]
pub(crate) struct Metering {
    unmetered_locals: usize,
    opcode_config: Arc<OpcodeConfig>,
    breakpoints_middleware: Arc<Breakpoints>,
    global_indexes: Mutex<Option<MeteringGlobalIndexes>>,
}

impl Metering {
    pub(crate) fn new(
        unmetered_locals: usize,
        opcode_config: Arc<OpcodeConfig>,
        breakpoints_middleware: Arc<Breakpoints>,
    ) -> Self {
        Self {
            unmetered_locals,
            opcode_config,
            breakpoints_middleware,
            global_indexes: Mutex::new(None),
        }
    }

    pub fn get_points_limit_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .points_limit_global_index
    }

    pub fn get_points_used_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .points_used_global_index
    }

    pub fn get_bulk_memory_size_operand_backup_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .bulk_memory_size_operand_backup_global_index
    }
}

unsafe impl Send for Metering {}
unsafe impl Sync for Metering {}

impl ModuleMiddleware for Metering {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionMetering {
            accumulated_cost: Default::default(),
            unmetered_locals: self.unmetered_locals,
            opcode_config: self.opcode_config.clone(),
            breakpoints_middleware: self.breakpoints_middleware.clone(),
            global_indexes: self.global_indexes.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) -> Result<(), MiddlewareError> {
        let mut global_indexes = self.global_indexes.lock().unwrap();

        *global_indexes = Some(MeteringGlobalIndexes {
            points_limit_global_index: create_global_index(
                module_info,
                METERING_POINTS_LIMIT,
                POINTS_LIMIT_INIT,
            ),
            points_used_global_index: create_global_index(module_info, METERING_POINTS_USED, 0),
            bulk_memory_size_operand_backup_global_index: create_global_index(
                module_info,
                METERING_BULK_MEMORY_SIZE_OPERAND_BACKUP,
                0,
            ),
        });

        Ok(())
    }
}

#[derive(Debug)]
struct FunctionMetering {
    accumulated_cost: u64,
    unmetered_locals: usize,
    opcode_config: Arc<OpcodeConfig>,
    breakpoints_middleware: Arc<Breakpoints>,
    global_indexes: MeteringGlobalIndexes,
}

impl FunctionMetering {
    fn inject_points_used_increment(&self, state: &mut MiddlewareReaderState) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_indexes.points_used_global_index.as_u32(),
            },
            Operator::I64Const {
                value: self.accumulated_cost as i64,
            },
            Operator::I64Add,
            Operator::GlobalSet {
                global_index: self.global_indexes.points_used_global_index.as_u32(),
            },
        ]);
    }

    fn inject_out_of_gas_check(&self, state: &mut MiddlewareReaderState) {
        state.extend(&[
            Operator::GlobalGet {
                global_index: self.global_indexes.points_used_global_index.as_u32(),
            },
            Operator::GlobalGet {
                global_index: self.global_indexes.points_limit_global_index.as_u32(),
            },
            Operator::I64GeU,
        ]);
        self.breakpoints_middleware
            .inject_breakpoint_condition(state, BREAKPOINT_VALUE_OUT_OF_GAS);
    }

    /// Injects `points_used += size * cost_per_byte` ahead of a `memory.copy`/`memory.fill`,
    /// using `size` as it appears on the stack (the raw, attacker-controlled operand), before
    /// the real bulk-memory instruction validates it against the actual memory bounds.
    ///
    /// This only covers the cost of the bytes. The flat cost of the instruction itself goes
    /// through the regular accumulator, so that a zero-size copy or fill is never free.
    ///
    /// The multiplication is plain wrapping `i64` arithmetic (wasm has no trapping or
    /// saturating integer multiply, and Wasmer doesn't offer one either), so it can in theory
    /// wrap around for a large enough `size`. This is not exploitable under the current setup:
    /// - `MAX_MEMORY_PAGES_ALLOWED` (in `we_instance.rs`) caps declared memory at 20 pages
    ///   (1.25 MiB), enforced at instantiation.
    /// - any `size` large enough to matter for overflow (`size * cost_per_byte` approaching
    ///   `i64::MAX`) necessarily exceeds that 1.25 MiB bound, so the real `memory.copy`/
    ///   `memory.fill` that follows this injected code traps on out-of-bounds access and aborts
    ///   the call immediately — regardless of what `points_used` ended up holding.
    /// - a `size` that keeps the copy in-bounds is capped at 1,310,720 bytes, so
    ///   `size * cost_per_byte` tops out around `5.6e15` for a `u32` cost, far below `i64::MAX`.
    fn inject_bulk_memory_cost(&self, state: &mut MiddlewareReaderState, cost_per_byte: u32) {
        // backup the bulk memory size
        state.extend(&[
            Operator::I64ExtendI32U,
            Operator::GlobalSet {
                global_index: self
                    .global_indexes
                    .bulk_memory_size_operand_backup_global_index
                    .as_u32(),
            },
        ]);

        // inject bulk memory cost
        state.extend(&[
            // memory size * price
            Operator::GlobalGet {
                global_index: self
                    .global_indexes
                    .bulk_memory_size_operand_backup_global_index
                    .as_u32(),
            },
            Operator::I64Const {
                value: cost_per_byte as i64,
            },
            Operator::I64Mul,
            // points used += memory size * price
            Operator::GlobalGet {
                global_index: self.global_indexes.points_used_global_index.as_u32(),
            },
            Operator::I64Add,
            Operator::GlobalSet {
                global_index: self.global_indexes.points_used_global_index.as_u32(),
            },
        ]);

        // bring back the bulk memory size
        state.extend(&[
            Operator::GlobalGet {
                global_index: self
                    .global_indexes
                    .bulk_memory_size_operand_backup_global_index
                    .as_u32(),
            },
            Operator::I32WrapI64,
        ]);
    }
}

impl FunctionMiddleware for FunctionMetering {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        // Get the cost of the current operator, and add it to the accumulator.
        // This needs to be done before the metering logic, to prevent operators like `Call` from escaping metering in some
        // corner cases.
        let op_exec_cost = get_opcode_cost(&operator, &self.opcode_config);
        match op_exec_cost {
            Cost::Illegal => {
                return Err(MiddlewareError::new(
                    "metering_middleware",
                    format!("Unsupported operator: {operator:?}"),
                ));
            }
            Cost::Base(base) => self.accumulated_cost += base as u64,
            Cost::BulkMemory { base, per_byte } => {
                self.accumulated_cost += base as u64;

                // flush what accumulated so far, including the base cost of this operator,
                // so that the out of gas check below takes all of it into account
                self.inject_points_used_increment(state);
                self.accumulated_cost = 0;

                self.inject_bulk_memory_cost(state, per_byte);

                // immediately insert out of gas check as this operation might be expensive
                self.inject_out_of_gas_check(state);
            }
        }

        if is_control_flow_operator(&operator) {
            self.inject_points_used_increment(state);
            self.inject_out_of_gas_check(state);

            self.accumulated_cost = 0;
        }

        state.push_operator(operator);

        Ok(())
    }

    // TODO: local count not available in Wasmer, options are:
    // a. find alternative
    // b. PR to Wasmer that gets accepted
    // c. fork Wasmer

    // fn feed_local_count(&mut self, count: u32) -> Result<(), MiddlewareError> {
    //     check_local_count_exceeded(count)?;
    //     let unmetered_locals = self.unmetered_locals as u32;
    //     if count > unmetered_locals {
    //         let metered_locals = count - unmetered_locals;
    //         let local_cost = self.opcode_cost.lock().unwrap().opcode_localallocate;
    //         let metered_locals_cost = metered_locals * local_cost;
    //         self.accumulated_cost += metered_locals_cost as u64;
    //     }
    //     Ok(())
    // }
}

pub(crate) fn get_points_limit(
    instance: &Instance,
    store: &mut impl AsStoreMut,
) -> Result<u64, ExecutorError> {
    get_global_value_u64(instance, store, METERING_POINTS_LIMIT)
}

pub(crate) fn set_points_limit(
    instance: &Instance,
    store: &mut impl AsStoreMut,
    limit: u64,
) -> Result<(), ExecutorError> {
    set_global_value_u64(instance, store, METERING_POINTS_LIMIT, limit)
}

pub(crate) fn get_points_used(
    instance: &Instance,
    store: &mut impl AsStoreMut,
) -> Result<u64, ExecutorError> {
    get_global_value_u64(instance, store, METERING_POINTS_USED)
}

pub(crate) fn set_points_used(
    instance: &Instance,
    store: &mut impl AsStoreMut,
    points: u64,
) -> Result<(), ExecutorError> {
    set_global_value_u64(instance, store, METERING_POINTS_USED, points)
}

fn check_local_count_exceeded(count: u32) -> Result<(), MiddlewareError> {
    if count > MAX_LOCAL_COUNT {
        return Err(MiddlewareError::new(
            "metering_middleware",
            format!("maximum number of locals({MAX_LOCAL_COUNT}) exceeded({count})"),
        ));
    }

    Ok(())
}
