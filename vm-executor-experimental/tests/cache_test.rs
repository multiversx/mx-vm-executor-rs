//! The serialized-artifact path: `Instance::cache` out, `Executor::new_instance_from_cache` in.
//!
//! The point of these tests is not that instantiation succeeds — that is the easy half — but
//! that the instance which comes back is *the same instance*. A serialized artifact carries
//! the middleware instrumentation baked in at compile time, and nothing re-runs the
//! middlewares on the way back. So the thing to pin is metering: if deserialization ever
//! stopped carrying the metering globals, contracts would run unmetered rather than fail, and
//! every other test here would still pass.

use multiversx_chain_vm_executor::{
    CompilationOptions, Executor, ExecutorError, ExecutorRuntimeDefault, Instance,
    InstanceCallResult, OpcodeConfig, OpcodeCost,
};
use multiversx_chain_vm_executor_wasmer_experimental::ExperimentalExecutor;
use std::sync::Arc;
use wasmer::wat2wasm;

const LOCAL_ALLOCATE_COST: u32 = 5;
const POINTS_LIMIT: u64 = 1_000_000;
const NUM_LOCALS: usize = 20;

/// Locals are the only priced opcode, so `points_used` after `init` is exactly
/// `NUM_LOCALS * LOCAL_ALLOCATE_COST` — a number that is wrong in an obvious way if the
/// metering middleware is missing (it would be 0) rather than merely different.
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

/// A contract with host imports and several exports, so that the introspection the Go VM runs
/// against every new instance — endpoint names, imported hook names, signature check — is
/// exercised on the restored side too.
fn imports_wasm() -> Vec<u8> {
    let wat = r#"(module
    (type $void (func))
    (type $i32_i32 (func (param i32 i32)))
    (import "env" "signalError" (func $signalError (type $i32_i32)))
    (import "env" "getGasLeft" (func $getGasLeft (result i64)))
    (func (export "init") (type $void))
    (func (export "add") (type $void))
    (func (export "getSum") (type $void))
    (func (export "callBack") (type $void))
    (memory (export "memory") 2))"#;
    wat2wasm(wat.as_bytes()).unwrap().into_owned()
}

fn executor() -> ExperimentalExecutor {
    ExperimentalExecutor::new(Arc::new(ExecutorRuntimeDefault))
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

/// The load-bearing one. Everything else here is a guard around it.
#[test]
fn cached_module_meters_identically() {
    let wasm = metered_wasm();
    let options = metered_options();

    let mut compiled = executor().new_instance(&wasm, &options).unwrap();
    let compiled_points = call_init(&mut compiled);

    // Not merely "equal to itself": the expected value is derived from the gas schedule, so a
    // round trip that silently dropped metering could not pass by also dropping it here.
    assert_eq!(
        compiled_points,
        NUM_LOCALS as u64 * LOCAL_ALLOCATE_COST as u64
    );

    let artifact = compiled.cache().unwrap();
    let mut restored = from_cache(&artifact).unwrap();

    assert_eq!(call_init(&mut restored), compiled_points);
}

#[test]
fn cached_module_keeps_its_endpoints_and_imports() {
    let wasm = imports_wasm();
    let options = CompilationOptions::default();

    let compiled = executor().new_instance(&wasm, &options).unwrap();
    let artifact = compiled.cache().unwrap();

    let restored = executor()
        .new_instance_from_cache(&artifact, &options)
        .unwrap();

    assert_eq!(
        restored.get_exported_function_names(),
        compiled.get_exported_function_names()
    );
    assert_eq!(
        restored.get_imported_function_names(),
        compiled.get_imported_function_names()
    );
    assert!(restored.has_function("add"));
    assert!(!restored.has_function("missingEndpoint"));
    assert!(restored.check_signatures());
}

/// A cached instance is a fresh instance, not a resumed one: its globals start where a newly
/// compiled one's would. This is what lets the Go VM treat the artifact cache as a substitute
/// for compiling, and it is the reason `reset()` is not needed — see
/// `PLAN-rust-vm-caching.md`, 1.4.
#[test]
fn cached_module_starts_fresh_each_time() {
    let artifact = executor()
        .new_instance(&metered_wasm(), &metered_options())
        .unwrap()
        .cache()
        .unwrap();

    let mut first = from_cache(&artifact).unwrap();
    let mut second = from_cache(&artifact).unwrap();

    // Run one of them twice over, so "fresh" is a claim about the second instance rather than
    // about an artifact that has simply not been used yet.
    let first_points = call_init(&mut first);
    let _ = first.call("init", POINTS_LIMIT);

    assert_eq!(call_init(&mut second), first_points);
}

/// An artifact can be re-serialized from an instance that was itself restored from one, so a
/// host is free to re-cache without going back to the wasm.
#[test]
fn cached_module_can_be_cached_again() {
    let artifact = executor()
        .new_instance(&metered_wasm(), &metered_options())
        .unwrap()
        .cache()
        .unwrap();

    let artifact_again = from_cache(&artifact).unwrap().cache().unwrap();

    let mut twice_restored = from_cache(&artifact_again).unwrap();
    assert_eq!(
        call_init(&mut twice_restored),
        NUM_LOCALS as u64 * LOCAL_ALLOCATE_COST as u64
    );
}

/// The artifact is Wasmer's own format, not a copy of the input. Worth stating because the Go
/// VM stores it under the *code hash*, so the two are easy to confuse at the call site.
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

/// Deserialization is unchecked in the sense that it does not *validate* the code, but it does
/// check the header, so a buffer that is not an artifact is an error rather than a jump into
/// arbitrary bytes. That is the only guarantee being claimed here — it is not a licence to
/// deserialize untrusted input.
#[test]
fn non_artifact_bytes_are_rejected() {
    let err = from_cache(b"not a wasmer artifact").err().unwrap();
    assert!(!err.to_string().is_empty());
}

/// The wasm itself is not an artifact. The two reach the executor through different entry
/// points, so feeding one to the other has to fail rather than quietly work.
#[test]
fn wasm_bytes_are_not_a_valid_artifact() {
    assert!(from_cache(&metered_wasm()).is_err());
}
