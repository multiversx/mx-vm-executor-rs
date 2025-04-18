use std::sync::Arc;

use multiversx_chain_vm_executor::{
    CompilationOptions, Instance, OpcodeCost, VMHooksBuilderDefault,
};
use multiversx_chain_vm_executor_wasmer_experimental::ExperimentalInstance;
use wasmer::wat2wasm;

const DUMMY_COMPILATION_OPTIONS: CompilationOptions = CompilationOptions {
    gas_limit: 0,
    unmetered_locals: 0,
    max_memory_grow: 0,
    max_memory_grow_delta: 0,
    opcode_trace: false,
    metering: false,
    runtime_breakpoints: false,
};

pub fn test_instance(wat: &str) -> Box<dyn Instance> {
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();

    Box::new(
        ExperimentalInstance::try_new_instance(
            Box::new(VMHooksBuilderDefault),
            Arc::new(OpcodeCost::default()),
            &wasm_bytes,
            &DUMMY_COMPILATION_OPTIONS,
        )
        .unwrap(),
    )
}
