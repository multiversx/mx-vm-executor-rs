use crate::get_opcode_cost;
use elrond_exec_service::OpcodeCost;
use loupe::{MemoryUsage, MemoryUsageTracker};
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;
use wasmer::wasmparser::{Operator, Type as WpType, TypeOrFuncType as WpTypeOrFuncType};
use wasmer::{
    ExportIndex, FunctionMiddleware, GlobalInit, GlobalType, Instance, LocalFunctionIndex,
    MiddlewareError, MiddlewareReaderState, ModuleMiddleware, Mutability, Type,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

const METERING_POINTS_LIMIT: &str = "metering_points_limit";
const METERING_POINTS_USED: &str = "metering_points_used";

#[derive(Clone, Debug, MemoryUsage)]
struct MeteringGlobalIndexes(GlobalIndex, GlobalIndex);

impl MeteringGlobalIndexes {
    fn points_limit(&self) -> GlobalIndex {
        self.0
    }

    fn points_used(&self) -> GlobalIndex {
        self.1
    }
}

#[derive(Debug)]
pub struct Metering {
    points_limit: u64,
    opcode_cost: Rc<OpcodeCost>,
    global_indexes: Rc<RefCell<Option<MeteringGlobalIndexes>>>,
}

impl Metering {
    pub fn new(points_limit: u64, opcode_cost: Rc<OpcodeCost>) -> Self {
        Self {
            points_limit,
            opcode_cost,
            global_indexes: Rc::new(RefCell::from(None)),
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
            opcode_cost: Rc::clone(&self.opcode_cost),
            global_indexes: self.global_indexes.borrow().clone().unwrap(),
        })
    }

    fn transform_module_info(&self, module_info: &mut ModuleInfo) {
        let points_limit_global_index = module_info
            .globals
            .push(GlobalType::new(Type::I64, Mutability::Var));

        module_info
            .global_initializers
            .push(GlobalInit::I64Const(self.points_limit as i64));

        module_info.exports.insert(
            METERING_POINTS_LIMIT.to_string(),
            ExportIndex::Global(points_limit_global_index),
        );

        let points_used_global_index = module_info
            .globals
            .push(GlobalType::new(Type::I32, Mutability::Var));

        module_info
            .global_initializers
            .push(GlobalInit::I32Const(0));

        module_info.exports.insert(
            METERING_POINTS_USED.to_string(),
            ExportIndex::Global(points_used_global_index),
        );

        let mut metering_global_indexes = self.global_indexes.borrow_mut();
        *metering_global_indexes = Some(MeteringGlobalIndexes(
            points_limit_global_index,
            points_used_global_index,
        ));
    }
}

#[derive(Debug)]
struct FunctionMetering {
    accumulated_cost: u64,
    opcode_cost: Rc<OpcodeCost>,
    global_indexes: MeteringGlobalIndexes,
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
        self.accumulated_cost += get_opcode_cost(&operator, self.opcode_cost.as_ref()) as u64;

        // Possible sources and targets of a branch. Finalize the cost of the previous basic block and perform necessary checks.
        match operator {
            Operator::Loop { .. } // loop headers are branch targets
            | Operator::End // block ends are branch targets
            | Operator::Else // "else" is the "end" of an if branch
            | Operator::Br { .. } // branch source
            | Operator::BrTable { .. } // branch source
            | Operator::BrIf { .. } // branch source
            | Operator::Call { .. } // function call - branch source
            | Operator::CallIndirect { .. } // function call - branch source
            | Operator::Return // end of function - branch source
            => {
                if self.accumulated_cost > 0 {
                    state.extend(&[
                        // Increment the points used counter.
                        Operator::GlobalGet { global_index: self.global_indexes.points_used().as_u32() },
                        Operator::I64Const { value: self.accumulated_cost as i64 },
                        Operator::I64Add,
                        Operator::GlobalSet { global_index: self.global_indexes.points_used().as_u32()},

                        // Check if out of gas (points_used >= points_limit)
                        Operator::GlobalGet { global_index: self.global_indexes.points_used().as_u32() },
                        Operator::GlobalGet { global_index: self.global_indexes.points_limit().as_u32() },
                        Operator::I64GeU,
                        Operator::If { ty: WpTypeOrFuncType::Type(WpType::EmptyBlockType) },
                        // TODO: insert breakpoint out of gas here
                        Operator::Unreachable,
                        Operator::End,
                    ]);

                    self.accumulated_cost = 0;
                }
            }
            _ => {}
        }
        state.push_operator(operator);

        Ok(())
    }
}

pub fn set_points_limit(instance: &Instance, limit: u64) {
    instance
        .exports
        .get_global(METERING_POINTS_LIMIT)
        .expect(format!("Can't get `{}` from Instance", METERING_POINTS_LIMIT).as_str())
        .set(limit.into())
        .expect(format!("Can't set `{}` in Instance", METERING_POINTS_LIMIT).as_str())
}

pub fn set_points_used(instance: &Instance, points: u64) {
    instance
        .exports
        .get_global(METERING_POINTS_USED)
        .expect(format!("Can't get `{}` from Instance", METERING_POINTS_USED).as_str())
        .set(points.into())
        .expect(format!("Can't set `{}` in Instance", METERING_POINTS_USED).as_str())
}

pub fn get_points_used(instance: &Instance) -> u64 {
    instance
        .exports
        .get_global(METERING_POINTS_USED)
        .expect(format!("Can't get `{}` from Instance", METERING_POINTS_USED).as_str())
        .get()
        .try_into()
        .expect(format!("`{}` from Instance has wrong type", METERING_POINTS_USED).as_str())
}
