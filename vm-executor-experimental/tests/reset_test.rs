//! `Instance::reset` on the Wasmer 7 backend.
//!
//! The implementation re-instantiates the module into a fresh store, so the thing to pin is
//! not that it runs but that it actually *clears* — every piece of per-execution state a
//! contract can leave behind has to be gone afterwards. A reset that silently kept state
//! would be worse than no reset at all: the host would reuse the instance believing it was
//! clean, and one execution's leftovers would be visible to the next.
//!
//! The checks are made **by the contract, not by the host**. This backend exposes neither
//! memory nor globals from outside a host call, and the globals here are deliberately not
//! exported — so nothing outside the module could observe them, and equally nothing outside
//! could have restored them by hand. That is precisely the state a host-side reset cannot
//! reach, and the reason this backend rebuilds the instance instead. See
//! `PLAN-reset-via-middlewares.md` for what it would take to do it in place.
//!
//! So each fixture exports an `expect_*` function that traps unless the state is what it
//! should be, and the assertion is on the call's outcome.

use multiversx_chain_vm_executor::{
    CompilationOptions, Executor, ExecutorRuntimeDefault, Instance, InstanceCallResult,
    OpcodeConfig, OpcodeCost,
};
use multiversx_chain_vm_executor_wasmer_experimental::ExperimentalExecutor;
use std::sync::Arc;
use wasmer::wat2wasm;

const POINTS_LIMIT: u64 = 1_000_000;

fn executor() -> ExperimentalExecutor {
    ExperimentalExecutor::new(Arc::new(ExecutorRuntimeDefault))
}

fn options() -> CompilationOptions {
    CompilationOptions {
        max_memory_grow: 10,
        max_memory_grow_delta: 10,
        ..Default::default()
    }
}

fn instance(wat: &str) -> Box<dyn Instance> {
    let wasm = wat2wasm(wat.as_bytes()).unwrap().into_owned();
    executor().new_instance(&wasm, &options()).unwrap()
}

/// The contract's own assertion passed.
fn call_ok(instance: &mut Box<dyn Instance>, func: &str) {
    match instance.call(func, POINTS_LIMIT) {
        InstanceCallResult::Ok => {}
        other => panic!("expected {func} to succeed, got {}", describe(&other)),
    }
}

/// The contract's own assertion failed, by trapping.
fn call_traps(instance: &mut Box<dyn Instance>, func: &str) {
    match instance.call(func, POINTS_LIMIT) {
        InstanceCallResult::RuntimeError(_) => {}
        other => panic!("expected {func} to trap, got {}", describe(&other)),
    }
}

fn describe(result: &InstanceCallResult) -> &'static str {
    match result {
        InstanceCallResult::Ok => "Ok",
        InstanceCallResult::FunctionNotFound => "FunctionNotFound",
        InstanceCallResult::RuntimeError(_) => "RuntimeError",
        InstanceCallResult::VMHooksEarlyExit(_) => "VMHooksEarlyExit",
        InstanceCallResult::Breakpoint(_) => "Breakpoint",
    }
}

/// A mutable global that is **not exported**, bumped on every call.
const COUNTER_WAT: &str = r#"
(module
    (memory (export "memory") 1)
    (global $counter (mut i32) (i32.const 0))
    (func (export "bump")
        (global.set $counter (i32.add (global.get $counter) (i32.const 1))))
    (func (export "expect_one")
        (if (i32.ne (global.get $counter) (i32.const 1)) (then unreachable)))
)
"#;

/// Memory with an active data segment, so a reset has to re-apply it rather than merely zero
/// the page. Offset 0 is covered by the segment; offset 64 is not.
const MEMORY_WAT: &str = r#"
(module
    (memory (export "memory") 1)
    (data (i32.const 0) "init")
    (func (export "scribble")
        (i32.store8 (i32.const 0) (i32.const 88))
        (i32.store8 (i32.const 64) (i32.const 99)))
    (func (export "expect_pristine")
        ;; 105 is 'i', the first byte of the data segment
        (if (i32.ne (i32.load8_u (i32.const 0)) (i32.const 105)) (then unreachable))
        (if (i32.ne (i32.load8_u (i32.const 64)) (i32.const 0)) (then unreachable)))
)
"#;

/// Grows its own memory, so a reset has to restore the *length*, not just the contents.
const GROW_WAT: &str = r#"
(module
    (memory (export "memory") 1)
    (func (export "grow") (drop (memory.grow (i32.const 1))))
    (func (export "expect_one_page")
        (if (i32.ne (memory.size) (i32.const 1)) (then unreachable)))
)
"#;

#[test]
fn reset_clears_an_unexported_mutable_global() {
    let mut instance = instance(COUNTER_WAT);

    call_ok(&mut instance, "bump");
    call_ok(&mut instance, "expect_one");

    call_ok(&mut instance, "bump");
    call_traps(&mut instance, "expect_one"); // now 2 — the fixture really does accumulate

    instance.reset().unwrap();

    call_ok(&mut instance, "bump");
    call_ok(&mut instance, "expect_one"); // back to 1, so the global was cleared
}

#[test]
fn reset_restores_the_memory_image() {
    let mut instance = instance(MEMORY_WAT);

    call_ok(&mut instance, "expect_pristine");
    call_ok(&mut instance, "scribble");
    call_traps(&mut instance, "expect_pristine");

    instance.reset().unwrap();

    // Both halves matter: the data segment is re-applied, and bytes it does not cover are
    // zero again. Zeroing alone would fail the first check; re-applying alone, the second.
    call_ok(&mut instance, "expect_pristine");
}

#[test]
fn reset_restores_the_memory_length() {
    let mut instance = instance(GROW_WAT);

    call_ok(&mut instance, "expect_one_page");
    call_ok(&mut instance, "grow");
    call_traps(&mut instance, "expect_one_page");

    instance.reset().unwrap();

    call_ok(&mut instance, "expect_one_page");
}

/// Priced options, since the default schedule is free and would make any gas assertion
/// vacuous. `global.set` is what `bump` does, so that is the one opcode that needs a cost.
fn metered_options() -> CompilationOptions {
    CompilationOptions {
        opcode_config: Arc::new(OpcodeConfig {
            opcode_cost: OpcodeCost {
                opcode_globalset: 10,
                ..Default::default()
            },
            ..Default::default()
        }),
        ..options()
    }
}

#[test]
fn reset_clears_the_gas_counter() {
    let wasm = wat2wasm(COUNTER_WAT.as_bytes()).unwrap().into_owned();
    let mut instance = executor().new_instance(&wasm, &metered_options()).unwrap();

    call_ok(&mut instance, "bump");
    assert!(
        instance.get_points_used().unwrap() > 0,
        "the call should have cost something"
    );

    instance.reset().unwrap();

    assert_eq!(
        instance.get_points_used().unwrap(),
        0,
        "the metering global survived the reset"
    );
}

/// A reset instance is the same instance: same module, same endpoints, still callable. The
/// host holds it across the reset, so its identity has to survive even though its state does
/// not.
#[test]
fn reset_keeps_the_instance_usable_and_identical() {
    let mut instance = instance(COUNTER_WAT);
    let exported = instance.get_exported_function_names();
    let imported = instance.get_imported_function_names();

    instance.reset().unwrap();

    assert_eq!(instance.get_exported_function_names(), exported);
    assert_eq!(instance.get_imported_function_names(), imported);
    assert!(instance.has_function("bump"));
    assert!(instance.check_signatures());

    call_ok(&mut instance, "bump");
}

#[test]
fn reset_is_repeatable() {
    let mut instance = instance(COUNTER_WAT);

    for _ in 0..3 {
        call_ok(&mut instance, "bump");
        call_ok(&mut instance, "expect_one");
        instance.reset().unwrap();
    }
}

/// An instance restored from a cached artifact resets like any other: the artifact carries
/// the module, and reset re-instantiates the module.
#[test]
fn reset_works_on_an_instance_from_cache() {
    let wasm = wat2wasm(COUNTER_WAT.as_bytes()).unwrap().into_owned();
    let artifact = executor()
        .new_instance(&wasm, &options())
        .unwrap()
        .cache()
        .unwrap();

    let mut instance = executor()
        .new_instance_from_cache(&artifact, &options())
        .unwrap();

    call_ok(&mut instance, "bump");
    call_ok(&mut instance, "bump");

    instance.reset().unwrap();

    call_ok(&mut instance, "bump");
    call_ok(&mut instance, "expect_one");
}
