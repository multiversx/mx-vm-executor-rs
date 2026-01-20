use wasmer::{
    ExportIndex, GlobalInit, GlobalType, Instance, Mutability, Type, wasmparser::Operator,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

pub trait MiddlewareWithProtectedGlobals {
    fn protected_globals(&self) -> Vec<u32>;
}

impl std::fmt::Debug for dyn MiddlewareWithProtectedGlobals {
    fn fmt(&self, _f: &mut core::fmt::Formatter) -> core::fmt::Result {
        Ok(())
    }
}

pub(crate) fn create_global_index(
    module_info: &mut ModuleInfo,
    key: &str,
    init: i64,
) -> GlobalIndex {
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
}

pub(crate) fn set_global_value_u64(
    instance: &Instance,
    global_name: &str,
    points: u64,
) -> Result<(), String> {
    let result = instance.exports.get_global(global_name);
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

pub(crate) fn get_global_value_u64(instance: &Instance, global_name: &str) -> Result<u64, String> {
    let result = instance.exports.get_global(global_name);
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

pub(crate) fn is_control_flow_operator(operator: &Operator) -> bool {
    matches!(
        operator,
        Operator::Loop { .. }
            | Operator::Block { .. }
            | Operator::End
            | Operator::If { .. }
            | Operator::Else
            | Operator::Unreachable
            | Operator::Br { .. }
            | Operator::BrTable { .. }
            | Operator::BrIf { .. }
            | Operator::Call { .. }
            | Operator::CallIndirect { .. }
            | Operator::Return
    )
}
