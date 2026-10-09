use multiversx_chain_vm_executor_wasmer::BasicExecutorService;
use multiversx_chain_vm_executor_wasmer::executor_interface::{
    CompilationOptionsLegacy, ExecutorError, ExecutorService, InstanceLegacy, VMHooksLegacyDefault,
    WASM_LOCALS_LIMIT,
};
use wasmer::wat2wasm;

const COMPILATION_OPTIONS: CompilationOptionsLegacy = CompilationOptionsLegacy {
    gas_limit: 0,
    unmetered_locals: 0,
    max_memory_grow: 0,
    max_memory_grow_delta: 0,
    opcode_trace: false,
    metering: false,
    runtime_breakpoints: false,
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

fn try_instance(num_locals: usize) -> Result<Box<dyn InstanceLegacy>, ExecutorError> {
    let wat = wat_with_locals(num_locals);
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();
    let service = BasicExecutorService::new();
    let executor = service
        .new_executor(Box::new(VMHooksLegacyDefault))
        .unwrap();
    executor.new_instance(&wasm_bytes, &COMPILATION_OPTIONS)
}

#[test]
fn deploy_maximum_locals_ok() {
    assert!(try_instance(WASM_LOCALS_LIMIT as usize).is_ok());
}

#[test]
fn deploy_more_than_maximum_locals_fails() {
    assert!(try_instance(WASM_LOCALS_LIMIT as usize + 1).is_err());
}
