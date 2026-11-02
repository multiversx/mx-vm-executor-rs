//! The serialized-artifact path on the Wasmer 2.2 backend, through the *new* traits.
//!
//! The same properties as the Wasmer 7 copy in `vm-executor-experimental/tests/cache_test.rs`,
//! asserted against the runtime that is in production. Both are worth having: the two
//! backends reach `new_instance_from_cache` through one trait but deserialize with two very
//! different Wasmer versions, and this is the one that has to keep matching what
//! `wasmer2legacy` has always done.

use multiversx_chain_vm_executor::{
    CompilationOptions, Executor, ExecutorError, ExecutorRuntimeDefault, Instance,
    InstanceCallResult, OpcodeConfig, OpcodeCost,
};
use multiversx_chain_vm_executor_wasmer::new_traits::WasmerProdExecutor;
use std::sync::Arc;
use wasmer::wat2wasm;

const LOCAL_ALLOCATE_COST: u32 = 5;
const POINTS_LIMIT: u64 = 1_000_000;
const NUM_LOCALS: usize = 20;

fn metered_options() -> CompilationOptions {
    CompilationOptions {
        unmetered_locals: 0,
        opcode_config: Arc::new(OpcodeConfig {
            opcode_cost: OpcodeCost {
                opcode_localallocate: LOCAL_ALLOCATE_COST,
                ..Default::default()
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn metered_wasm() -> Vec<u8> {
    let wat = format!(
        r#"(module
    (func (export "init")
        (local {}))
    (memory (export "memory") 17))"#,
        "i32 ".repeat(NUM_LOCALS)
    );
    wat2wasm(wat.as_bytes()).unwrap().into_owned()
}

fn executor() -> WasmerProdExecutor {
    WasmerProdExecutor::new(Arc::new(ExecutorRuntimeDefault))
}

fn call_init(instance: &mut Box<dyn Instance>) -> u64 {
    assert!(matches!(
        instance.call("init", POINTS_LIMIT),
        InstanceCallResult::Ok
    ));
    instance.get_points_used().unwrap()
}

fn from_cache(cache_bytes: &[u8]) -> Result<Box<dyn Instance>, ExecutorError> {
    executor().new_instance_from_cache(cache_bytes, &metered_options())
}

/// Metering has to survive the round trip, or contracts restored from cache run free.
#[test]
fn cached_module_meters_identically() {
    let wasm = metered_wasm();
    let options = metered_options();

    let mut compiled = executor().new_instance(&wasm, &options).unwrap();
    let compiled_points = call_init(&mut compiled);

    assert_eq!(
        compiled_points,
        NUM_LOCALS as u64 * LOCAL_ALLOCATE_COST as u64
    );

    let artifact = compiled.cache().unwrap();
    let mut restored = from_cache(&artifact).unwrap();

    assert_eq!(call_init(&mut restored), compiled_points);
}

#[test]
fn cached_module_keeps_its_endpoints() {
    let wasm = metered_wasm();
    let options = metered_options();

    let compiled = executor().new_instance(&wasm, &options).unwrap();
    let artifact = compiled.cache().unwrap();
    let restored = from_cache(&artifact).unwrap();

    assert_eq!(
        restored.get_exported_function_names(),
        compiled.get_exported_function_names()
    );
    assert!(restored.has_function("init"));
    assert!(restored.check_signatures());
}

#[test]
fn artifact_is_not_the_wasm() {
    let wasm = metered_wasm();
    let artifact = executor()
        .new_instance(&wasm, &metered_options())
        .unwrap()
        .cache()
        .unwrap();

    assert_ne!(artifact, wasm);
    assert!(!artifact.is_empty());
}

#[test]
fn empty_artifact_is_rejected() {
    let err = from_cache(&[]).err().unwrap();
    assert!(err.to_string().contains("empty"), "unexpected error: {err}");
}

#[test]
fn wasm_bytes_are_not_a_valid_artifact() {
    assert!(from_cache(&metered_wasm()).is_err());
}
