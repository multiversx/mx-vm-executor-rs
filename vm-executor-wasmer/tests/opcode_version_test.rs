use multiversx_chain_vm_executor::{
    CompilationOptions, ExecutorLegacy, OpcodeConfig, OpcodeCost, OpcodeVersion,
    VMHooksLegacyDefault,
};
use multiversx_chain_vm_executor_wasmer::WasmerExecutor;
use wasmer::wat2wasm;

const MEMORY_COPY_WAT: &[u8] = include_bytes!("memory-copy.wat");

#[test]
fn opcode_version_test() {
    let wasm_bytes = wat2wasm(MEMORY_COPY_WAT).unwrap();

    let mut executor = WasmerExecutor::new(Box::new(VMHooksLegacyDefault));
    let result = executor.new_instance(&wasm_bytes, &CompilationOptions::default().to_legacy());
    assert!(result.is_err());

    executor
        .set_opcode_config(OpcodeConfig {
            opcode_version: OpcodeVersion::V2,
            opcode_cost: OpcodeCost::default(),
        })
        .unwrap();

    let result = executor.new_instance(&wasm_bytes, &CompilationOptions::default().to_legacy());
    assert!(result.is_ok());
}
