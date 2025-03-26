use std::rc::Rc;

use multiversx_chain_vm_executor::{CompilationOptions, Executor, Instance, VMHooksBuilderDefault};
use multiversx_chain_vm_executor_wasmer_experimental::WasmerExecutor;
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
    let executor = WasmerExecutor::new(Rc::new(VMHooksBuilderDefault));

    // service.new_executor(Box::new(VMHooksDefault)).unwrap();

    executor
        .new_instance(&wasm_bytes, &DUMMY_COMPILATION_OPTIONS)
        .unwrap()
}
