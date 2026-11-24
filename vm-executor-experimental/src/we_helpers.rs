use multiversx_chain_vm_executor::ExecutorError;
use wasmer::{
    AsStoreMut, ExportIndex, GlobalInit, GlobalType, Instance, Mutability, Type,
    wasmparser::Operator,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

use crate::ExperimentalError;

/// Creates and exports a fresh mutable i32 global in the module and initializes it.
pub(crate) fn create_i32_global_index(
    module_info: &mut ModuleInfo,
    key: &str,
    init: i32,
) -> GlobalIndex {
    create_global_index(module_info, key, Type::I32, GlobalInit::I32Const(init))
}

/// Creates and exports a fresh mutable i64 global in the module and initializes it.
pub(crate) fn create_i64_global_index(
    module_info: &mut ModuleInfo,
    key: &str,
    init: i64,
) -> GlobalIndex {
    create_global_index(module_info, key, Type::I64, GlobalInit::I64Const(init))
}

/// Only reachable through the typed constructors above, which are what keeps the declared
/// type and the initializer from disagreeing.
fn create_global_index(
    module_info: &mut ModuleInfo,
    key: &str,
    global_type: Type,
    init: GlobalInit,
) -> GlobalIndex {
    let global_index = module_info
        .globals
        .push(GlobalType::new(global_type, Mutability::Var));

    module_info.global_initializers.push(init);

    module_info
        .exports
        .insert(key.to_string(), ExportIndex::Global(global_index));

    global_index
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

pub(crate) fn get_global_value_u64(
    instance: &Instance,
    store: &mut impl AsStoreMut,
    global_name: &'static str,
) -> Result<u64, ExecutorError> {
    let global = instance
        .exports
        .get_global(global_name)
        .map_err(|err| ExperimentalError::GlobalAccess(global_name, err))?;
    let value = global.get(store);
    value
        .try_into()
        .map_err(|err| ExperimentalError::ParseGlobalValue(global_name, err).into())
}

pub(crate) fn set_global_value_u64(
    instance: &Instance,
    store: &mut impl AsStoreMut,
    global_name: &'static str,
    value: u64,
) -> Result<(), ExecutorError> {
    let global = instance
        .exports
        .get_global(global_name)
        .map_err(|err| ExperimentalError::GlobalAccess(global_name, err))?;
    global
        .set(store, value.into())
        .map_err(|err| ExperimentalError::SetGlobalValue(global_name, err).into())
}
