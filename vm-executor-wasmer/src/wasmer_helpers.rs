use wasmer::{
    ExportIndex, GlobalInit, GlobalType, Instance, Mutability, Type, wasmparser::Operator,
};
use wasmer_types::{GlobalIndex, ModuleInfo};

/// Middleware that marks some globals as protected and therefore not subject to
/// normal runtime mutation.
pub trait MiddlewareWithProtectedGlobals {
    /// Returns the indices of globals that must remain protected.
    fn protected_globals(&self) -> Vec<u32>;
}

impl std::fmt::Debug for dyn MiddlewareWithProtectedGlobals {
    fn fmt(&self, _f: &mut core::fmt::Formatter) -> core::fmt::Result {
        Ok(())
    }
}

/// Creates and exports a fresh i64 global in the module and initializes it.
///
/// # Parameters
/// - `module_info`: the module metadata to update
/// - `key`: the exported name for the global
/// - `init`: the initial value assigned to the global
///
/// # Returns
/// The index of the newly created global in the module.
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

/// Sets the value of a u64 export global on a Wasmer instance.
///
/// # Parameters
/// - `instance`: the instance that owns the exported global
/// - `global_name`: the name of the exported global
/// - `points`: the new value to assign to the global
///
/// # Returns
/// `Ok(())` if the update succeeded, otherwise a formatted error message.
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

/// Reads the value of a u64 export global from a Wasmer instance.
///
/// # Parameters
/// - `instance`: the instance that owns the exported global
/// - `global_name`: the name of the exported global
///
/// # Returns
/// The current value of the global as a `u64`, or an error message if lookup
/// or conversion fails.
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

/// Returns `true` when the given WebAssembly operator changes control flow.
///
/// This is used to recognize branching and call-like operations that affect block
/// execution flow and must be treated specially by middleware logic.
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

/// Returns `true` for the bulk-memory operators supported by this helper.
///
/// These are the memory-copy and memory-fill instructions that need explicit
/// validation or handling in Wasmer middleware.
pub(crate) fn is_supported_bulk_memory_operator(operator: &Operator) -> bool {
    matches!(
        operator,
        Operator::MemoryCopy { .. } | Operator::MemoryFill { .. }
    )
}
