use multiversx_chain_vm_executor::ExecutorError;
use wasmer::{
    wasmparser::Operator, AsStoreMut, ExportIndex, GlobalInit, GlobalType, Instance, Mutability,
    Type,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

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

pub(crate) fn set_global_value_u64(
    instance: &Instance,
    store: &mut impl AsStoreMut,
    global_name: &str,
    value: u64,
) -> anyhow::Result<()> {
    let global = instance.exports.get_global(global_name)?;
    Ok(global.set(store, value.into())?)
}

pub(crate) fn get_global_value_u64(
    instance: &Instance,
    store: &mut impl AsStoreMut,
    global_name: &str,
) -> anyhow::Result<u64> {
    let global = instance.exports.get_global(global_name)?;
    let value = global.get(store);
    value
        .try_into()
        .map_err(|err| anyhow::anyhow!("error parsing global value: {err}"))
}
