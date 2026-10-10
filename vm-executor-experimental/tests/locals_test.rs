use multiversx_chain_vm_executor::{
    CompilationOptions, ExecutorError, Instance, InstanceCallResult, OpcodeConfig, OpcodeCost,
    WASM_LOCALS_LIMIT,
};
use multiversx_chain_vm_executor_wasmer_experimental::{
    ExperimentalInstance, ExperimentalVMHooksBuilderDefault,
};
use std::sync::Arc;
use wasmer::wat2wasm;

const COMPILATION_OPTIONS: CompilationOptions = CompilationOptions {
    unmetered_locals: 0,
    max_memory_grow: 0,
    max_memory_grow_delta: 0,
    opcode_trace: false,
};

fn wat_with_locals(num_locals: usize) -> String {
    format!(
        r#"(module
    (func (export "init")
        (local {}))
    (memory (export "memory") 17))"#,
        "i32 ".repeat(num_locals)
    )
}

fn try_instance(num_locals: usize) -> Result<Box<dyn Instance>, ExecutorError> {
    let wat = wat_with_locals(num_locals);
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
    ExperimentalInstance::try_new_instance(
        Box::new(ExperimentalVMHooksBuilderDefault),
        Arc::new(OpcodeConfig::default()),
        &wasm_bytes,
        &COMPILATION_OPTIONS,
    )
    .map(|instance| Box::new(instance) as Box<dyn Instance>)
}

#[test]
fn deploy_maximum_locals_ok() {
    assert!(try_instance(WASM_LOCALS_LIMIT as usize).is_ok());
}

#[test]
fn deploy_more_than_maximum_locals_fails() {
    assert!(try_instance(WASM_LOCALS_LIMIT as usize + 1).is_err());
}

const LOCAL_ALLOCATE_COST: u32 = 5;
const POINTS_LIMIT: u64 = 1_000_000;

/// All opcodes are free except local allocation, so `points_used` after calling `init`
/// is exactly the cost of the metered locals.
fn points_used_with_locals(num_locals: usize, unmetered_locals: usize) -> u64 {
    let wat = wat_with_locals(num_locals);
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
    let opcode_config = OpcodeConfig {
        opcode_cost: OpcodeCost {
            opcode_localallocate: LOCAL_ALLOCATE_COST,
            ..Default::default()
        },
        ..Default::default()
    };
    let options = CompilationOptions {
        unmetered_locals,
        ..COMPILATION_OPTIONS
    };

    let mut instance = ExperimentalInstance::try_new_instance(
        Box::new(ExperimentalVMHooksBuilderDefault),
        Arc::new(opcode_config),
        &wasm_bytes,
        &options,
    )
    .unwrap();
    assert!(matches!(
        instance.call("init", POINTS_LIMIT),
        InstanceCallResult::Ok
    ));
    instance.get_points_used().unwrap()
}

#[test]
fn locals_below_or_at_unmetered_are_free() {
    assert_eq!(points_used_with_locals(0, 10), 0);
    assert_eq!(points_used_with_locals(9, 10), 0);
    assert_eq!(points_used_with_locals(10, 10), 0);
}

#[test]
fn locals_above_unmetered_are_charged() {
    assert_eq!(points_used_with_locals(11, 10), LOCAL_ALLOCATE_COST as u64);
    assert_eq!(
        points_used_with_locals(25, 10),
        15 * LOCAL_ALLOCATE_COST as u64
    );
    assert_eq!(
        points_used_with_locals(WASM_LOCALS_LIMIT as usize, 0),
        WASM_LOCALS_LIMIT as u64 * LOCAL_ALLOCATE_COST as u64
    );
}

/// A threshold that does not fit in `u32` must not wrap around to a small value.
#[test]
fn huge_unmetered_locals_exempts_everything() {
    assert_eq!(
        points_used_with_locals(WASM_LOCALS_LIMIT as usize, 1 << 32),
        0
    );
}
