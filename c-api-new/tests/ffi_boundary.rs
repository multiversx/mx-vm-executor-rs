//! Drives the C API from Rust, so the boundary is testable before any Go exists.
//!
//! Two halves:
//!
//! - [`lifecycle`] goes through the exported functions exactly as the Go side will, using a
//!   mock hook table.
//! - [`state_ops`] installs a mock hooks provider that calls the state functions from inside
//!   a real host call, which is the only place a hook context is live. It also checks that a
//!   context which outlives its call, or which belongs to an outer call while a nested one is
//!   running, is rejected rather than followed.

use std::ffi::{CString, c_char, c_int, c_void};
use std::sync::{Arc, Mutex};

use multiversx_chain_vm_executor::{VMHooks, VMHooksDefault};
use multiversx_chain_vm_executor_c_api_new::capi_config::{
    vm_exec2_compilation_options_t, vm_exec2_executor_config_t,
};
use multiversx_chain_vm_executor_c_api_new::capi_executor::{
    vm_exec2_executor_destroy, vm_exec2_executor_t, vm_exec2_new_executor,
};
use multiversx_chain_vm_executor_c_api_new::capi_hook_context::{
    CapiVMHooksBuilder, vm_exec2_hook_context_t,
};
use multiversx_chain_vm_executor_c_api_new::capi_hook_table::CapiHookTable;
use multiversx_chain_vm_executor_c_api_new::capi_instance::*;
use multiversx_chain_vm_executor_c_api_new::capi_instance_state::*;
use multiversx_chain_vm_executor_c_api_new::capi_opcode_cost::vm_exec_opcode_cost_t;
use multiversx_chain_vm_executor_c_api_new::capi_status::vm_exec2_status_t;
use multiversx_chain_vm_executor_wasmer_experimental::{
    ExperimentalInstance, ExperimentalInstanceState, ExperimentalVMHooksBuilder,
};

/// One memory page, five known bytes at offset 0, one endpoint that calls a VM hook.
const CONTRACT_WAT: &str = r#"
(module
    (import "env" "getGasLeft" (func $getGasLeft (result i64)))
    (memory (export "memory") 1)
    (data (i32.const 0) "hello")
    (func (export "doNothing"))
    (func (export "callHook") (drop (call $getGasLeft)))
)
"#;

const ERR_BUF_LEN: c_int = 1024;

struct ErrBuf(Vec<c_char>);

impl ErrBuf {
    fn new() -> Self {
        ErrBuf(vec![0; ERR_BUF_LEN as usize])
    }

    fn ptr(&mut self) -> *mut c_char {
        self.0.as_mut_ptr()
    }

    fn message(&self) -> String {
        let bytes: Vec<u8> = self
            .0
            .iter()
            .take_while(|b| **b != 0)
            .map(|b| *b as u8)
            .collect();
        String::from_utf8(bytes).expect("error message is not valid UTF-8")
    }
}

fn opcode_cost() -> vm_exec_opcode_cost_t {
    // A `#[repr(C)]` struct of `u32`s, so an all-zero cost table is a valid value. Zero
    // costs keep the tests away from the metering limit, which is not what they are about.
    unsafe { std::mem::zeroed() }
}

fn compilation_options() -> vm_exec2_compilation_options_t {
    vm_exec2_compilation_options_t {
        unmetered_locals: 0,
        max_memory_grow: 10,
        max_memory_grow_delta: 10,
        opcode_trace: false,
    }
}

/// A mock hook table: plausible non-null words, never called in this phase.
fn mock_hook_table() -> Vec<*const c_void> {
    (1..=8usize).map(|n| n as *const c_void).collect()
}

fn new_executor(err: &mut ErrBuf) -> *mut vm_exec2_executor_t {
    let cost = opcode_cost();
    let config = vm_exec2_executor_config_t {
        opcode_version: 1,
        opcode_cost: &cost,
    };
    let table = mock_hook_table();

    let mut executor: *mut vm_exec2_executor_t = std::ptr::null_mut();
    let status = unsafe {
        vm_exec2_new_executor(
            &mut executor,
            &config,
            table.as_ptr() as *const c_void,
            table.len(),
            err.ptr(),
            ERR_BUF_LEN,
        )
    };

    assert_eq!(
        status,
        vm_exec2_status_t::VM_EXEC2_OK,
        "new_executor failed: {}",
        err.message()
    );
    assert!(!executor.is_null());
    executor
}

fn new_instance(executor: *mut vm_exec2_executor_t, err: &mut ErrBuf) -> *mut vm_exec2_instance_t {
    let options = compilation_options();
    let wasm = CONTRACT_WAT.as_bytes();
    let go_vm_hooks = 0xBEEFusize as *mut c_void;

    let mut instance: *mut vm_exec2_instance_t = std::ptr::null_mut();
    let status = unsafe {
        vm_exec2_new_instance(
            executor,
            &mut instance,
            wasm.as_ptr(),
            wasm.len() as u32,
            &options,
            go_vm_hooks,
            err.ptr(),
            ERR_BUF_LEN,
        )
    };

    assert_eq!(
        status,
        vm_exec2_status_t::VM_EXEC2_OK,
        "new_instance failed: {}",
        err.message()
    );
    assert!(!instance.is_null());
    instance
}

fn call(
    instance: *mut vm_exec2_instance_t,
    func_name: &str,
    err: &mut ErrBuf,
) -> vm_exec2_call_result_t {
    let name = CString::new(func_name).unwrap();
    let mut result = vm_exec2_call_result_t {
        kind: vm_exec2_call_result_kind_t::VM_EXEC2_CALL_RUNTIME_ERROR,
        early_exit_code: u64::MAX,
        breakpoint: u64::MAX,
    };

    let status = unsafe {
        vm_exec2_instance_call(
            instance,
            name.as_ptr(),
            1_000_000,
            &mut result,
            err.ptr(),
            ERR_BUF_LEN,
        )
    };
    assert_eq!(status, vm_exec2_status_t::VM_EXEC2_OK);
    result
}

fn names(
    instance: *mut vm_exec2_instance_t,
    length: unsafe extern "C" fn(*const vm_exec2_instance_t) -> c_int,
    write: unsafe extern "C" fn(*const vm_exec2_instance_t, *mut c_char, c_int) -> c_int,
) -> Vec<String> {
    let needed = unsafe { length(instance) };
    assert!(needed > 0, "expected a non-empty name list");

    let mut buf = vec![0 as c_char; needed as usize];
    let written = unsafe { write(instance, buf.as_mut_ptr(), needed) };
    assert_eq!(written, needed, "two-step length and write must agree");

    let bytes: Vec<u8> = buf
        .iter()
        .take_while(|b| **b != 0)
        .map(|b| *b as u8)
        .collect();
    let mut names: Vec<String> = String::from_utf8(bytes)
        .unwrap()
        .split('|')
        .map(str::to_string)
        .collect();
    names.sort();
    names
}

#[test]
fn lifecycle() {
    let mut err = ErrBuf::new();
    let executor = new_executor(&mut err);
    let instance = new_instance(executor, &mut err);

    // A call that does nothing succeeds.
    let result = call(instance, "doNothing", &mut err);
    assert_eq!(result.kind, vm_exec2_call_result_kind_t::VM_EXEC2_CALL_OK);

    // A call into a VM hook succeeds too; with no generated dispatch yet, the hooks are the
    // executor's own placeholder implementation.
    let result = call(instance, "callHook", &mut err);
    assert_eq!(result.kind, vm_exec2_call_result_kind_t::VM_EXEC2_CALL_OK);

    // A missing endpoint is reported in the result, not as a failure of the call itself.
    let result = call(instance, "noSuchEndpoint", &mut err);
    assert_eq!(
        result.kind,
        vm_exec2_call_result_kind_t::VM_EXEC2_CALL_FUNCTION_NOT_FOUND
    );

    // Gas accounting is readable from outside a host call.
    let mut points = u64::MAX;
    let status =
        unsafe { vm_exec2_instance_get_points_used(instance, &mut points, err.ptr(), ERR_BUF_LEN) };
    assert_eq!(status, vm_exec2_status_t::VM_EXEC2_OK);
    assert_ne!(points, u64::MAX, "points_used was not written");

    // Every endpoint takes no arguments and returns nothing.
    assert_eq!(unsafe { vm_exec2_instance_check_signatures(instance) }, 1);

    let present = CString::new("doNothing").unwrap();
    let absent = CString::new("noSuchEndpoint").unwrap();
    assert_eq!(
        unsafe { vm_exec2_instance_has_function(instance, present.as_ptr()) },
        1
    );
    assert_eq!(
        unsafe { vm_exec2_instance_has_function(instance, absent.as_ptr()) },
        0
    );

    assert_eq!(
        names(
            instance,
            vm_exec2_instance_exported_function_names_length,
            vm_exec2_instance_exported_function_names,
        ),
        vec!["callHook".to_string(), "doNothing".to_string()]
    );

    // All of the hooks the module imports, in one call.
    assert_eq!(
        names(
            instance,
            vm_exec2_instance_imported_function_names_length,
            vm_exec2_instance_imported_function_names,
        ),
        vec!["getGasLeft".to_string()]
    );

    unsafe {
        vm_exec2_instance_destroy(instance);
        vm_exec2_executor_destroy(executor);
    }
}

#[test]
fn null_arguments_are_refused_without_a_message() {
    let mut err = ErrBuf::new();
    let mut executor: *mut vm_exec2_executor_t = std::ptr::null_mut();

    // A null config cannot be reported through the error buffer, because the buffer itself
    // may be the thing that is null, so these return a dedicated status instead.
    let status = unsafe {
        vm_exec2_new_executor(
            &mut executor,
            std::ptr::null(),
            std::ptr::null(),
            0,
            err.ptr(),
            ERR_BUF_LEN,
        )
    };
    assert_eq!(status, vm_exec2_status_t::VM_EXEC2_NULL_POINTER);
    assert!(executor.is_null());

    assert_eq!(
        unsafe { vm_exec2_instance_check_signatures(std::ptr::null()) },
        vm_exec2_status_t::VM_EXEC2_NULL_POINTER.as_c_int()
    );
}

#[test]
fn an_invalid_opcode_version_is_reported_through_the_buffer() {
    let mut err = ErrBuf::new();
    let cost = opcode_cost();
    let config = vm_exec2_executor_config_t {
        opcode_version: 99,
        opcode_cost: &cost,
    };

    let mut executor: *mut vm_exec2_executor_t = std::ptr::null_mut();
    let status = unsafe {
        vm_exec2_new_executor(
            &mut executor,
            &config,
            std::ptr::null(),
            0,
            err.ptr(),
            ERR_BUF_LEN,
        )
    };

    assert_eq!(status, vm_exec2_status_t::VM_EXEC2_ERROR);
    assert_eq!(err.message(), "invalid opcode version code: 99");
    assert!(executor.is_null());
}

#[test]
fn a_context_is_rejected_when_no_call_is_active() {
    let mut err = ErrBuf::new();

    assert_eq!(
        unsafe {
            vm_exec2_state_mem_length(std::ptr::null_mut(), &mut 0u64, err.ptr(), ERR_BUF_LEN)
        },
        vm_exec2_status_t::VM_EXEC2_NULL_POINTER
    );

    let mut points = 0u64;
    assert_eq!(
        unsafe {
            vm_exec2_state_get_points_used(
                std::ptr::null_mut(),
                &mut points,
                err.ptr(),
                ERR_BUF_LEN,
            )
        },
        vm_exec2_status_t::VM_EXEC2_NULL_POINTER
    );
}

/// What `state_ops` records from inside the host call, to be asserted after it returns.
#[derive(Default)]
struct Outcome {
    failures: Vec<String>,
    hook_calls: usize,
    escaped_context: usize,
}

impl Outcome {
    fn check(&mut self, what: &str, condition: bool, detail: String) {
        if !condition {
            self.failures.push(format!("{what}: {detail}"));
        }
    }
}

/// A mock hooks provider that exercises the state FFI from inside a real host call.
struct ProbingHooksBuilder {
    inner: CapiVMHooksBuilder,
    outcome: Arc<Mutex<Outcome>>,
}

impl ExperimentalVMHooksBuilder for ProbingHooksBuilder {
    fn with_vm_hooks(
        &self,
        instance_state_ref: &mut ExperimentalInstanceState,
        f: &mut dyn FnMut(&mut dyn VMHooks),
    ) {
        let state_ptr = (instance_state_ref as *mut ExperimentalInstanceState<'_>).cast::<c_void>();
        let mut outcome = self.outcome.lock().unwrap();
        outcome.hook_calls += 1;

        unsafe {
            self.inner.with_hook_context_raw(state_ptr, |ctx| {
                probe_state_ops(ctx, &mut outcome);
                probe_nesting(&self.inner, state_ptr, ctx, &mut outcome);
                outcome.escaped_context = ctx as usize;
            });
        }

        drop(outcome);
        f(&mut VMHooksDefault);
    }
}

fn mem_load(ctx: *mut vm_exec2_hook_context_t, mem_ptr: i32, dest: &mut [u8], len: i32) -> i64 {
    unsafe {
        vm_exec2_state_mem_load(
            ctx,
            mem_ptr,
            dest.as_mut_ptr(),
            len,
            std::ptr::null_mut(),
            0,
        )
    }
}

fn mem_store(ctx: *mut vm_exec2_hook_context_t, mem_ptr: i32, src: &[u8]) -> vm_exec2_status_t {
    unsafe {
        vm_exec2_state_mem_store(
            ctx,
            mem_ptr,
            src.as_ptr(),
            src.len() as i32,
            std::ptr::null_mut(),
            0,
        )
    }
}

fn probe_state_ops(ctx: *mut vm_exec2_hook_context_t, outcome: &mut Outcome) {
    const PAGE: u64 = 65536;

    let mut length = 0u64;
    let status = unsafe { vm_exec2_state_mem_length(ctx, &mut length, std::ptr::null_mut(), 0) };
    outcome.check(
        "mem_length",
        status == vm_exec2_status_t::VM_EXEC2_OK && length == PAGE,
        format!("{status:?}, length {length}"),
    );

    // A load entirely inside the memory returns what the data segment put there.
    let mut buf = [0u8; 5];
    let loaded = mem_load(ctx, 0, &mut buf, 5);
    outcome.check(
        "mem_load reads the data segment",
        loaded == 5 && &buf == b"hello",
        format!("loaded {loaded}, {:?}", std::str::from_utf8(&buf)),
    );

    // A load that runs past the end is truncated, not refused: the Go VM's semantics.
    let mut buf = [0u8; 10];
    let loaded = mem_load(ctx, (PAGE - 2) as i32, &mut buf, 10);
    outcome.check(
        "mem_load clamps at the end of memory",
        loaded == 2,
        format!("loaded {loaded}"),
    );

    // An offset exactly at the end is a valid offset that yields nothing.
    let loaded = mem_load(ctx, PAGE as i32, &mut buf, 1);
    outcome.check(
        "mem_load at the end of memory yields nothing",
        loaded == 0,
        format!("loaded {loaded}"),
    );

    // An offset past the end is an error, even though a too-long range is not.
    let loaded = mem_load(ctx, (PAGE + 1) as i32, &mut buf, 1);
    outcome.check(
        "mem_load rejects an offset past the end",
        loaded == vm_exec2_status_t::VM_EXEC2_ERROR.as_i64(),
        format!("loaded {loaded}"),
    );

    let loaded = mem_load(ctx, -1, &mut buf, 1);
    outcome.check(
        "mem_load rejects a negative offset",
        loaded == vm_exec2_status_t::VM_EXEC2_ERROR.as_i64(),
        format!("loaded {loaded}"),
    );

    let loaded = mem_load(ctx, 0, &mut buf, -1);
    outcome.check(
        "mem_load rejects a negative length",
        loaded == vm_exec2_status_t::VM_EXEC2_ERROR.as_i64(),
        format!("loaded {loaded}"),
    );

    // A store, read back through a load.
    let status = mem_store(ctx, 16, b"ABC");
    let mut buf = [0u8; 3];
    let loaded = mem_load(ctx, 16, &mut buf, 3);
    outcome.check(
        "mem_store round-trips",
        status == vm_exec2_status_t::VM_EXEC2_OK && loaded == 3 && &buf == b"ABC",
        format!(
            "{status:?}, loaded {loaded}, {:?}",
            std::str::from_utf8(&buf)
        ),
    );

    // Unlike a load, a store is never truncated.
    let status = mem_store(ctx, (PAGE - 1) as i32, b"ABC");
    outcome.check(
        "mem_store rejects a range past the end",
        status == vm_exec2_status_t::VM_EXEC2_ERROR,
        format!("{status:?}"),
    );

    // Gas, through the state handle rather than the instance handle.
    let status = unsafe { vm_exec2_state_set_points_used(ctx, 42, std::ptr::null_mut(), 0) };
    let mut points = 0u64;
    let get_status =
        unsafe { vm_exec2_state_get_points_used(ctx, &mut points, std::ptr::null_mut(), 0) };
    outcome.check(
        "points used round-trip",
        status == vm_exec2_status_t::VM_EXEC2_OK
            && get_status == vm_exec2_status_t::VM_EXEC2_OK
            && points == 42,
        format!("{status:?}, {get_status:?}, points {points}"),
    );

    // Growing reports the page count from before the growth.
    let mut prev_pages = 0u32;
    let status =
        unsafe { vm_exec2_state_mem_grow(ctx, 1, &mut prev_pages, std::ptr::null_mut(), 0) };
    let mut length_after = 0u64;
    unsafe { vm_exec2_state_mem_length(ctx, &mut length_after, std::ptr::null_mut(), 0) };
    outcome.check(
        "mem_grow",
        status == vm_exec2_status_t::VM_EXEC2_OK && prev_pages == 1 && length_after == 2 * PAGE,
        format!("{status:?}, prev_pages {prev_pages}, length {length_after}"),
    );
}

/// A nested host call must not let the outer call's context stay usable.
fn probe_nesting(
    builder: &CapiVMHooksBuilder,
    state_ptr: *mut c_void,
    outer_ctx: *mut vm_exec2_hook_context_t,
    outcome: &mut Outcome,
) {
    unsafe {
        builder.with_hook_context_raw(state_ptr, |inner_ctx| {
            outcome.check(
                "nesting uses a distinct context",
                inner_ctx != outer_ctx,
                format!("inner {inner_ctx:?} outer {outer_ctx:?}"),
            );

            let mut length = 0u64;
            let inner_status =
                vm_exec2_state_mem_length(inner_ctx, &mut length, std::ptr::null_mut(), 0);
            outcome.check(
                "the innermost context works",
                inner_status == vm_exec2_status_t::VM_EXEC2_OK,
                format!("{inner_status:?}"),
            );

            let outer_status =
                vm_exec2_state_mem_length(outer_ctx, &mut length, std::ptr::null_mut(), 0);
            outcome.check(
                "an outer context is rejected while a nested call runs",
                outer_status == vm_exec2_status_t::VM_EXEC2_STALE_CONTEXT,
                format!("{outer_status:?}"),
            );
        });

        // ... and is usable again once the nested call has returned.
        let mut length = 0u64;
        let status = vm_exec2_state_mem_length(outer_ctx, &mut length, std::ptr::null_mut(), 0);
        outcome.check(
            "the outer context works again after the nested call",
            status == vm_exec2_status_t::VM_EXEC2_OK,
            format!("{status:?}"),
        );
    }
}

#[test]
fn state_ops() {
    let outcome = Arc::new(Mutex::new(Outcome::default()));
    let builder = ProbingHooksBuilder {
        inner: CapiVMHooksBuilder::new(0xBEEFusize as *mut c_void, unsafe {
            CapiHookTable::copy_from(std::ptr::null(), 0)
        }),
        outcome: outcome.clone(),
    };

    let mut instance = ExperimentalInstance::try_new_instance(
        Box::new(builder),
        Arc::new(Default::default()),
        CONTRACT_WAT.as_bytes(),
        // The same options as the C path, built directly: the FFI mirror is crate-private.
        &multiversx_chain_vm_executor::CompilationOptions {
            unmetered_locals: 0,
            max_memory_grow: 10,
            max_memory_grow_delta: 10,
            opcode_trace: false,
        },
    )
    .expect("instance creation failed");

    use multiversx_chain_vm_executor::Instance;
    let result = instance.call("callHook", 1_000_000);
    assert!(
        matches!(result, multiversx_chain_vm_executor::InstanceCallResult::Ok),
        "the probing call did not succeed"
    );

    let outcome = outcome.lock().unwrap();
    assert_eq!(
        outcome.hook_calls, 1,
        "the hook was not called exactly once"
    );
    assert!(
        outcome.failures.is_empty(),
        "state op failures:\n  {}",
        outcome.failures.join("\n  ")
    );

    // The context that escaped the host call must now be refused. Contexts are pooled for
    // the instance's lifetime, so this reads live memory and fails the token check, rather
    // than being undefined behaviour.
    let escaped = outcome.escaped_context as *mut vm_exec2_hook_context_t;
    let mut length = 0u64;
    let status =
        unsafe { vm_exec2_state_mem_length(escaped, &mut length, std::ptr::null_mut(), 0) };
    assert_eq!(
        status,
        vm_exec2_status_t::VM_EXEC2_STALE_CONTEXT,
        "a context used after its call must be rejected"
    );
}
