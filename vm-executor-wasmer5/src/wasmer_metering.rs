use crate::wasmer_breakpoints::{Breakpoints, BREAKPOINT_VALUE_OUT_OF_GAS};
use crate::wasmer_helpers::{
    create_global_index, is_control_flow_operator, MiddlewareWithProtectedGlobals,
};
use crate::{get_local_cost, get_opcode_cost};
use loupe::{MemoryUsage, MemoryUsageTracker};
use multiversx_chain_vm_executor::OpcodeCost;
use std::mem;
use std::sync::{Arc, Mutex};
use wasmer::wasmparser::Operator;
use wasmer::{
    FunctionMiddleware, Instance, LocalFunctionIndex, MiddlewareError, MiddlewareReaderState,
    ModuleMiddleware,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

const METERING_POINTS_LIMIT: &str = "metering_points_limit";
const METERING_POINTS_USED: &str = "metering_points_used";
const MAX_LOCAL_COUNT: u32 = 4000;

#[derive(Clone, Debug, MemoryUsage)]
struct MeteringGlobalIndexes {
    points_limit_global_index: GlobalIndex,
    points_used_global_index: GlobalIndex,
}

#[derive(Debug)]
pub(crate) struct Metering {
    points_limit: u64,
    unmetered_locals: usize,
    opcode_cost: Arc<Mutex<OpcodeCost>>,
    breakpoints_middleware: Arc<Breakpoints>,
    global_indexes: Mutex<Option<MeteringGlobalIndexes>>,
}

impl Metering {
    pub(crate) fn new(
        points_limit: u64,
        unmetered_locals: usize,
        opcode_cost: Arc<Mutex<OpcodeCost>>,
        breakpoints_middleware: Arc<Breakpoints>,
    ) -> Self {
        Self {
            points_limit,
            unmetered_locals,
            opcode_cost,
            breakpoints_middleware,
            global_indexes: Mutex::new(None),
        }
    }

    fn get_points_limit_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .points_limit_global_index
    }

    fn get_points_used_global_index(&self) -> GlobalIndex {
        self.global_indexes
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .points_used_global_index
    }
}

unsafe impl Send for Metering {}
unsafe impl Sync for Metering {}

impl MemoryUsage for Metering {
    fn size_of_val(&self, tracker: &mut dyn MemoryUsageTracker) -> usize {
        mem::size_of_val(self) + self.global_indexes.size_of_val(tracker)
            - mem::size_of_val(&self.global_indexes)
    }
}

impl ModuleMiddleware for Metering {
    fn generate_function_middleware(
        &self,
        _local_function_index: LocalFunctionIndex,
    ) -> Box<dyn FunctionMiddleware> {
        Box::new(FunctionMetering {
            accumulated_cost: Default::default(),
            unmetered_locals: self.unmetered_locals,
            opcode_cost: self.opcode_cost.clone(),
            breakpoints_middleware: self.breakpoints_middleware.clone(),
            global_indexes: self.global_indexes.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_indexes = self.global_indexes.lock().unwrap();

        let points_limit = self.points_limit as i64;

        *global_indexes = Some(MeteringGlobalIndexes {
            points_limit_global_index: create_global_index(
                module_info,
                METERING_POINTS_LIMIT,
                points_limit,
            ),
            points_used_global_index: create_global_index(module_info, METERING_POINTS_USED, 0),
        });
    }
}

impl MiddlewareWithProtectedGlobals for Metering {
    fn protected_globals(&self) -> Vec<u32> {
        vec![
            self.get_points_limit_global_index().as_u32(),
            self.get_points_used_global_index().as_u32(),
        ]
    }
}

#[derive(Debug)]
struct FunctionMetering {
    accumulated_cost: u64,
    unmetered_locals: usize,
    opcode_cost: Arc<Mutex<OpcodeCost>>,
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
        let option = get_opcode_cost(&operator, &self.opcode_cost.lock().unwrap());
        match option {
            Some(cost) => self.accumulated_cost += cost as u64,
            None => {
                return Err(MiddlewareError::new(
                    "metering_middleware",
                    format!("Unsupported operator: {operator:?}"),
                ))
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

    fn feed_local_count(&mut self, count: u32) -> Result<(), MiddlewareError> {
        check_local_count_exceeded(count)?;

        let unmetered_locals = self.unmetered_locals as u32;
        if count > unmetered_locals {
            let metered_locals = count - unmetered_locals;
            let local_cost = get_local_cost(&self.opcode_cost.lock().unwrap());
            let metered_locals_cost = metered_locals * local_cost;
            self.accumulated_cost += metered_locals_cost as u64;
        }

        Ok(())
    }
}

pub(crate) fn set_points_limit(instance: &Instance, limit: u64) -> Result<(), String> {
    let result = instance.exports.get_global(METERING_POINTS_LIMIT);
    match result {
        Ok(global) => {
            let result = global.set(limit.into());
            match result {
                Ok(_) => Ok(()),
                Err(err) => Err(err.message()),
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

pub(crate) fn set_points_used(instance: &Instance, points: u64) -> Result<(), String> {
    let result = instance.exports.get_global(METERING_POINTS_USED);
    match result {
        Ok(global) => {
            let result = global.set(points.into());
            match result {
                Ok(_) => Ok(()),
                Err(err) => Err(err.message()),
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

pub(crate) fn get_points_used(instance: &Instance) -> Result<u64, String> {
    let result = instance.exports.get_global(METERING_POINTS_USED);
    match result {
        Ok(global) => {
            let result = global.get().try_into();
            match result {
                Ok(points) => Ok(points),
                Err(err) => Err(err.to_string()),
            }
        }
        Err(err) => Err(err.to_string()),
    }
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
