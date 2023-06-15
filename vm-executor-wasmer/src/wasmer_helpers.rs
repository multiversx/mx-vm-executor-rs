use wasmer::{ExportIndex, GlobalInit, GlobalType, Mutability, Type};
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
