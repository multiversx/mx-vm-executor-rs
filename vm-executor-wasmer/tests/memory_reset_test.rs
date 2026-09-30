//! Warm-instance reuse resets the instance in place. For static memories
//! (the only kind the executor produces) the reset must keep the
//! `4 GiB bound + 2 GiB guard` reservation and the memory base address, since
//! singlepass emits no explicit bounds checks and relies on that guard.

mod common;

use multiversx_chain_vm_executor_wasmer::executor_interface::CompilationOptionsLegacy;

const WASM_PAGE: u32 = 65536;

const COMPILATION_OPTIONS: CompilationOptionsLegacy = CompilationOptionsLegacy {
    gas_limit: 1_000_000_000,
    unmetered_locals: 0,
    max_memory_grow: 20,
    max_memory_grow_delta: 10,
    opcode_trace: false,
    metering: false,
    runtime_breakpoints: false,
};

/// Same module as `test/contracts/memory-reset` in mx-chain-vm-go.
const MEMORY_RESET_WAT: &str = r#"
(module
  (memory (export "memory") 1 20)
  (data (i32.const 0) "\01\02\03\04")
  (func (export "init"))
  (func (export "callBack"))
  (func (export "growAndDirty")
    (drop (memory.grow (i32.const 3)))
    (i32.store (i32.const 196608) (i32.const 0x5eed5eed)))
  (func (export "touchMinimum")
    (drop (i32.load (i32.const 0))))
  (func (export "loadGrownPage")
    (drop (i32.load (i32.const 196608))))
  (func (export "loadHigh")
    (drop (i32.load (i32.const 0xC0000000))))
  (func (export "loadLast")
    (drop (i32.load (i32.const 0xFFFFFFFC)))))
"#;

#[test]
fn reset_keeps_static_memory_guard_and_base() {
    let instance = common::test_instance_with_options(MEMORY_RESET_WAT, &COMPILATION_OPTIONS);
    let base = instance.memory_ptr().unwrap();
    assert_eq!(instance.memory_length().unwrap(), WASM_PAGE as u64);

    // Grow from inside wasm and dirty the last new page.
    instance.call("growAndDirty").unwrap();
    assert_eq!(instance.memory_length().unwrap(), 4 * WASM_PAGE as u64);
    assert_eq!(
        instance.memory_load(3 * WASM_PAGE as isize, 4).unwrap(),
        0x5eed5eedu32.to_le_bytes()
    );
    instance.call("loadGrownPage").unwrap();

    // This is what the Go host does on warm-instance reuse.
    instance.reset().unwrap();

    assert_eq!(instance.memory_length().unwrap(), WASM_PAGE as u64);
    assert_eq!(
        instance.memory_ptr().unwrap(),
        base,
        "reset must not move a static memory"
    );
    assert_eq!(
        instance.memory_load(0, 4).unwrap(),
        [1, 2, 3, 4],
        "data segments are re-applied"
    );

    // Everything above the minimum must trap again: the page that was grown,
    // an address deep inside the 4 GiB bound, and the last word.
    for endpoint in ["loadGrownPage", "loadHigh", "loadLast"] {
        let err = instance
            .call(endpoint)
            .expect_err("access past the current memory size must trap");
        assert!(
            err.contains("out of bounds memory access"),
            "{endpoint}: unexpected error {err:?}"
        );
    }

    // Growing again reuses the same mapping and yields zeroed pages.
    assert_eq!(instance.memory_grow(3).unwrap(), 1);
    assert_eq!(instance.memory_ptr().unwrap(), base);
    assert_eq!(
        instance.memory_load(3 * WASM_PAGE as isize, 4).unwrap(),
        [0; 4]
    );
    instance.call("loadGrownPage").unwrap();
}

#[test]
fn reset_is_idempotent_at_minimum() {
    let instance = common::test_instance_with_options(MEMORY_RESET_WAT, &COMPILATION_OPTIONS);
    let base = instance.memory_ptr().unwrap();
    instance.memory_store(100, &[0xAA, 0xBB]).unwrap();

    instance.reset().unwrap();
    instance.reset().unwrap();

    assert_eq!(instance.memory_ptr().unwrap(), base);
    assert_eq!(instance.memory_length().unwrap(), WASM_PAGE as u64);
    assert_eq!(instance.memory_load(100, 2).unwrap(), [0, 0]);
    assert_eq!(instance.memory_load(0, 4).unwrap(), [1, 2, 3, 4]);
}
