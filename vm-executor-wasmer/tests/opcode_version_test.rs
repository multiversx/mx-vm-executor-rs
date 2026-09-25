use multiversx_chain_vm_executor::{
    CompilationOptions, ExecutorLegacy, OpcodeConfig, OpcodeCost, OpcodeVersion,
    VMHooksLegacyDefault,
};
use multiversx_chain_vm_executor_wasmer::WasmerExecutor;
use wasmer::wat2wasm;

const MEMORY_COPY_WAT: &[u8] = include_bytes!("memory-copy.wat");

const DUMMY_COMPILATION_OPTIONS: CompilationOptions = CompilationOptions {
    unmetered_locals: 0,
    max_memory_grow: 0,
    max_memory_grow_delta: 0,
    opcode_trace: false,
};

#[test]
fn opcode_version_test() {
    let wasm_bytes = wat2wasm(MEMORY_COPY_WAT).unwrap();

    let mut executor = WasmerExecutor::new(Box::new(VMHooksLegacyDefault));
    let result = executor.new_instance(&wasm_bytes, &DUMMY_COMPILATION_OPTIONS.to_legacy());
    assert!(result.is_err());

    executor
        .set_opcode_config(OpcodeConfig {
            opcode_version: OpcodeVersion::V2,
            opcode_cost: OpcodeCost::default(),
        })
        .unwrap();

    let result = executor.new_instance(&wasm_bytes, &DUMMY_COMPILATION_OPTIONS.to_legacy());
    assert!(result.is_ok());
}
