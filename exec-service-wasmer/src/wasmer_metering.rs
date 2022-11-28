use crate::get_opcode_cost;
#[allow(unused_imports)]
use crate::wasmer_breakpoints::{Breakpoints, BREAKPOINT_VALUE_OUT_OF_GAS};
use elrond_exec_service::OpcodeCost;
use loupe::{MemoryUsage, MemoryUsageTracker};
use std::mem;
use std::sync::{Arc, Mutex};
use wasmer::wasmparser::Operator;
use wasmer::{
    ExportIndex, FunctionMiddleware, GlobalInit, GlobalType, Instance, LocalFunctionIndex,
    MiddlewareError, MiddlewareReaderState, ModuleMiddleware, Mutability, Type,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

const METERING_POINTS_LIMIT: &str = "metering_points_limit";
const METERING_POINTS_USED: &str = "metering_points_used";

#[derive(Clone, Debug, MemoryUsage)]
struct MeteringGlobalIndexes {
    points_limit_global_index: GlobalIndex,
    points_used_global_index: GlobalIndex,
}

#[derive(Debug)]
pub(crate) struct Metering {
    points_limit: u64,
    opcode_cost: Arc<OpcodeCost>,
    breakpoints_middleware: Arc<Breakpoints>,
    global_indexes: Mutex<Option<MeteringGlobalIndexes>>,
}

impl Metering {
    pub(crate) fn new(
        points_limit: u64,
        opcode_cost: Arc<OpcodeCost>,
        breakpoints_middleware: Arc<Breakpoints>,
    ) -> Self {
        unsafe {
            LIMIT_POINTS = points_limit;
        }
        Self {
            points_limit,
            opcode_cost,
            breakpoints_middleware,
            global_indexes: Mutex::new(None),
        }
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
            opcode_cost: self.opcode_cost.clone(),
            breakpoints_middleware: self.breakpoints_middleware.clone(),
            global_indexes: self.global_indexes.lock().unwrap().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let mut global_indexes = self.global_indexes.lock().unwrap();

        let mut create_global_index = |key: &str, init: i64| {
            let global_index = module_info
                .globals
                .push(GlobalType::new(Type::I64, Mutability::Var));

            module_info
                .global_initializers
                .push(GlobalInit::I64Const(init));

            module_info
                .exports
                .insert(key.to_string(), ExportIndex::Global(global_index));

            global_index
        };

        let points_limit = self.points_limit as i64;

        *global_indexes = Some(MeteringGlobalIndexes {
            points_limit_global_index: create_global_index(METERING_POINTS_LIMIT, points_limit),
            points_used_global_index: create_global_index(METERING_POINTS_USED, 0),
        });
    }
}

#[derive(Debug)]
struct FunctionMetering {
    accumulated_cost: u64,
    opcode_cost: Arc<OpcodeCost>,
    breakpoints_middleware: Arc<Breakpoints>,
    global_indexes: MeteringGlobalIndexes,
}

impl FunctionMetering {
    fn inject_points_used_increment<'b>(&self, state: &mut MiddlewareReaderState<'b>) {
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

    fn inject_out_of_gas_check<'b>(&self, state: &mut MiddlewareReaderState<'b>) {
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

#[allow(dead_code)]
static mut ACCUMULATED_POINTS: u64 = 0;
static mut LIMIT_POINTS: u64 = 0;

impl FunctionMiddleware for FunctionMetering {
    fn feed<'b>(
        &mut self,
        operator: Operator<'b>,
        state: &mut MiddlewareReaderState<'b>,
    ) -> Result<(), MiddlewareError> {
        // Get the cost of the current operator, and add it to the accumulator.
        // This needs to be done before the metering logic, to prevent operators like `Call` from escaping metering in some
        // corner cases.
        self.accumulated_cost += get_opcode_cost(&operator, &self.opcode_cost) as u64;

        if matches!(
            operator,
            Operator::Loop { .. }
                | Operator::End
                | Operator::Else
                | Operator::Br { .. }
                | Operator::BrTable { .. }
                | Operator::BrIf { .. }
                | Operator::Call { .. }
                | Operator::CallIndirect { .. }
                | Operator::Return
        ) {
            // println!("incrementing with {} points", self.accumulated_cost);
            // unsafe {
            //     ACCUMULATED_POINTS += self.accumulated_cost;
            //     println!("ACCUMULATED_POINTS: {}", ACCUMULATED_POINTS);
            //     println!("LIMIT_POINTS: {}", LIMIT_POINTS);
            //     if ACCUMULATED_POINTS >= LIMIT_POINTS {
            //         println!("----- OUT OF GAS -----");
            //     }
            // }
            self.inject_points_used_increment(state);
            self.inject_out_of_gas_check(state);

            self.accumulated_cost = 0;
        }

        state.push_operator(operator);

        Ok(())
    }
}

pub(crate) fn set_points_limit(instance: &Instance, limit: u64) {
    // println!("setting points_limit: {}", limit);
    instance
        .exports
        .get_global(METERING_POINTS_LIMIT)
        .unwrap_or_else(|_| panic!("Can't get `{}` from Instance", METERING_POINTS_LIMIT))
        .set(limit.into())
        .unwrap_or_else(|_| panic!("Can't set `{}` in Instance", METERING_POINTS_LIMIT))
}

pub(crate) fn set_points_used(instance: &Instance, points: u64) {
    // println!("setting points_used: {}", points);
    instance
        .exports
        .get_global(METERING_POINTS_USED)
        .unwrap_or_else(|_| panic!("Can't get `{}` from Instance", METERING_POINTS_USED))
        .set(points.into())
        .unwrap_or_else(|_| panic!("Can't set `{}` in Instance", METERING_POINTS_USED))
}

pub(crate) fn get_points_used(instance: &Instance) -> u64 {
    let point_used = instance
        .exports
        .get_global(METERING_POINTS_USED)
        .unwrap_or_else(|_| panic!("Can't get `{}` from Instance", METERING_POINTS_USED))
        .get()
        .try_into()
        .unwrap_or_else(|_| panic!("`{}` from Instance has wrong type", METERING_POINTS_USED));
    // println!("getting points_used: {}", point_used);
    point_used
}
