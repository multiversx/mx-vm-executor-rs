use std::sync::Arc;

use multiversx_chain_vm_executor::{
    CompilationOptions, Instance, InstanceCallResult, OpcodeConfig, OpcodeCost, OpcodeVersion,
};
use multiversx_chain_vm_executor_wasmer_experimental::{
    ExperimentalInstance, ExperimentalVMHooksBuilderDefault,
};
use wasmer::wat2wasm;

const BULK_MEMORY_WAT: &[u8] = include_bytes!("bulk-memory.wat");

const POINTS_LIMIT: u64 = 1_000_000;
const MEMORY_COPY_COST: u32 = 300;
const MEMORY_COPY_PER_BYTE_COST: u32 = 7;
const MEMORY_FILL_COST: u32 = 400;
const MEMORY_FILL_PER_BYTE_COST: u32 = 11;

const COMPILATION_OPTIONS: CompilationOptions = CompilationOptions {
    unmetered_locals: 0,
    max_memory_grow: 0,
    max_memory_grow_delta: 0,
    opcode_trace: false,
};

/// All opcodes are free, except the bulk memory ones, so that `points_used` after a call
/// is exactly the cost of the single `memory.copy`/`memory.fill` in the called function.
///
/// A fresh instance per call, since `points_used` only starts at zero.
fn points_used_by(func_name: &str) -> u64 {
    let wasm_bytes = wat2wasm(BULK_MEMORY_WAT).unwrap();

    let opcode_config = OpcodeConfig {
        opcode_version: OpcodeVersion::V2,
        opcode_cost: OpcodeCost {
            opcode_memorycopy: MEMORY_COPY_COST,
            opcode_memorycopyperbyte: MEMORY_COPY_PER_BYTE_COST,
            opcode_memoryfill: MEMORY_FILL_COST,
            opcode_memoryfillperbyte: MEMORY_FILL_PER_BYTE_COST,
            ..Default::default()
        },
    };

    let mut instance = ExperimentalInstance::try_new_instance(
        Box::new(ExperimentalVMHooksBuilderDefault),
        Arc::new(opcode_config),
        &wasm_bytes,
        &COMPILATION_OPTIONS,
    )
    .unwrap();

    assert!(matches!(
        instance.call(func_name, POINTS_LIMIT),
        InstanceCallResult::Ok
    ));

    instance.get_points_used().unwrap()
}

#[test]
fn bulk_memory_zero_bytes_still_costs_the_base() {
    assert_eq!(points_used_by("copyZero"), MEMORY_COPY_COST as u64);
    assert_eq!(points_used_by("fillZero"), MEMORY_FILL_COST as u64);
}

#[test]
fn bulk_memory_bytes_cost_on_top_of_the_base() {
    assert_eq!(
        points_used_by("copyTen"),
        (MEMORY_COPY_COST + 10 * MEMORY_COPY_PER_BYTE_COST) as u64
    );
    assert_eq!(
        points_used_by("fillTen"),
        (MEMORY_FILL_COST + 10 * MEMORY_FILL_PER_BYTE_COST) as u64
    );
}

/// The injected metering code takes the size operand off the stack and puts it back.
#[test]
fn bulk_memory_size_operand_survives_the_injection() {
    assert_eq!(points_used_by("copyLocalTen"), points_used_by("copyTen"));
}
