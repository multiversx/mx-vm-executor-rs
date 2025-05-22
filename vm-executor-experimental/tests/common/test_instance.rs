use std::sync::Arc;

use multiversx_chain_vm_executor::{CompilationOptions, Instance, OpcodeCost};
use multiversx_chain_vm_executor_wasmer_experimental::{
    ExperimentalInstance, ExperimentalVMHooksBuilderDefault,
};
use wasmer::wat2wasm;

const DUMMY_COMPILATION_OPTIONS: CompilationOptions = CompilationOptions {
    unmetered_locals: 0,
    max_memory_grow: 0,
    max_memory_grow_delta: 0,
    opcode_trace: false,
};

pub fn test_instance(wat: &str) -> Box<dyn Instance> {
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();

    Box::new(
        ExperimentalInstance::try_new_instance(
            Box::new(ExperimentalVMHooksBuilderDefault),
            Arc::new(OpcodeCost::default()),
            &wasm_bytes,
            &DUMMY_COMPILATION_OPTIONS,
        )
        .unwrap(),
    )
}
