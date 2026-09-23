use multiversx_chain_vm_executor::{
    CompilationOptionsLegacy, ExecutorLegacy, InstanceLegacy, OpcodeConfig, OpcodeCost,
    OpcodeVersion, VMHooksLegacyDefault,
};
use multiversx_chain_vm_executor_wasmer::WasmerExecutor;
use wasmer::wat2wasm;

const BULK_MEMORY_WAT: &[u8] = include_bytes!("bulk-memory.wat");

const MEMORY_COPY_COST: u32 = 300;
const MEMORY_COPY_PER_BYTE_COST: u32 = 7;
const MEMORY_FILL_COST: u32 = 400;
const MEMORY_FILL_PER_BYTE_COST: u32 = 11;

const COMPILATION_OPTIONS: CompilationOptionsLegacy = CompilationOptionsLegacy {
    gas_limit: 1_000_000,
    unmetered_locals: 0,
    max_memory_grow: 0,
    max_memory_grow_delta: 0,
    opcode_trace: false,
    metering: true,
    runtime_breakpoints: true,
};

/// All opcodes are free, except the bulk memory ones, so that `points_used` after a call
/// is exactly the cost of the single `memory.copy`/`memory.fill` in the called function.
fn bulk_memory_instance() -> Box<dyn InstanceLegacy> {
    let wasm_bytes = wat2wasm(BULK_MEMORY_WAT).unwrap();

    let mut executor = WasmerExecutor::new(Box::new(VMHooksLegacyDefault));
    executor
        .set_opcode_config(OpcodeConfig {
            opcode_version: OpcodeVersion::V2,
            opcode_cost: OpcodeCost {
                opcode_memorycopy: MEMORY_COPY_COST,
                opcode_memorycopyperbyte: MEMORY_COPY_PER_BYTE_COST,
                opcode_memoryfill: MEMORY_FILL_COST,
                opcode_memoryfillperbyte: MEMORY_FILL_PER_BYTE_COST,
                ..Default::default()
            },
        })
        .unwrap();

    executor
        .new_instance(&wasm_bytes, &COMPILATION_OPTIONS)
        .unwrap()
}

fn points_used_by(instance: &dyn InstanceLegacy, func_name: &str) -> u64 {
    instance.set_points_used(0).unwrap();
    instance.call(func_name).unwrap();
    instance.get_points_used().unwrap()
}

#[test]
fn bulk_memory_zero_bytes_still_costs_the_base() {
    let instance = bulk_memory_instance();

    assert_eq!(
        points_used_by(instance.as_ref(), "copyZero"),
        MEMORY_COPY_COST as u64
    );
    assert_eq!(
        points_used_by(instance.as_ref(), "fillZero"),
        MEMORY_FILL_COST as u64
    );
}

#[test]
fn bulk_memory_bytes_cost_on_top_of_the_base() {
    let instance = bulk_memory_instance();

    assert_eq!(
        points_used_by(instance.as_ref(), "copyTen"),
        (MEMORY_COPY_COST + 10 * MEMORY_COPY_PER_BYTE_COST) as u64
    );
    assert_eq!(
        points_used_by(instance.as_ref(), "fillTen"),
        (MEMORY_FILL_COST + 10 * MEMORY_FILL_PER_BYTE_COST) as u64
    );
}
