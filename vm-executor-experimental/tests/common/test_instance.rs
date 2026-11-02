use std::sync::Arc;

use multiversx_chain_vm_executor::{
    CompilationOptions, Executor, ExecutorRuntimeDefault, Instance,
};
use multiversx_chain_vm_executor_wasmer_experimental::ExperimentalExecutor;
use wasmer::wat2wasm;

pub fn test_instance(wat: &str) -> Box<dyn Instance> {
    let wasm_bytes = wat2wasm(wat.as_bytes()).unwrap();

    ExperimentalExecutor::new(Arc::new(ExecutorRuntimeDefault))
        .new_instance(&wasm_bytes, &CompilationOptions::default())
        .unwrap()
}
