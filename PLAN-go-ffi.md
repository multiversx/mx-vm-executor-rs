# Integrating the new executor traits into the Go VM

Working plan for exposing `vm-executor/src/new_traits` over a C FFI and consuming it from
the Go VM. Status as of 2026-10-01: Phase 1 items 1-5 and Phase 2 are implemented.

## Why

The repo was built around Wasmer 2.2, hidden behind the legacy interfaces in
`vm-executor`. Those interfaces cannot express Wasmer 6's ownership model for VM hooks, so a
second set of traits was added in `vm-executor/src/new_traits`, currently driven only by the
Rust VM (`mx-sdk-rs/chain/vm`). The Go VM still speaks the legacy C API. This plan covers
the new FFI crate, the generator work, and the Go-side changes.

Design principles: configure the executor directly and all at once; prefer pure functions;
one config object rather than a sequence of setters.

## Where things live

| Piece | Path |
| --- | --- |
| Interfaces, legacy + new | `vm-executor/src/` and `vm-executor/src/new_traits/` |
| Wasmer 2.2 backend (legacy native, new traits via a shim) | `vm-executor-wasmer/` |
| Wasmer 6 backend (new traits native) | `vm-executor-experimental/` |
| Current C API (legacy only) | `c-api/` |
| New C API (new traits, Wasmer 6) | `c-api-new/` |
| Code generator (drives **both** repos) | `mx-chain-vm-go/vmhost/vmhooks/generate/` |
| Go VM | `mx-chain-vm-go/`, cgo bridge in `wasmer2/` |
| Rust VM, Wasmer 6 consumer | `mx-sdk-rs/chain/vm/src/executor_impl/we_executor.rs` |
| Rust VM, Wasmer 2.2 consumer | `mx-sdk-rs/chain/wasmer-prod/src/wasmer_prod.rs` |

## How the two interfaces differ

| | Legacy | New |
| --- | --- | --- |
| `Executor` | 4 methods, 2 of them post-construction setters | 2 instantiation methods; everything else at construction |
| `CompilationOptions` | 7 fields | 4; `gas_limit` moved to `call`, metering and breakpoints always on |
| Instance | one 17-method trait | `Instance` (7, outside view) + `InstanceState` (inside view, for hooks) |
| Call result | `Result<(), String>` plus breakpoint globals to poll | `InstanceCallResult` enum |
| Breakpoints | 6 values | 3; `ExecutionFailed`/`SignalError`/`AsyncCall` became `VMHooksEarlyExit` |
| VM hooks | `&self`, bare returns, carries a context pointer | `&mut self`, `Result<_, VMHooksEarlyExit>`, no context pointer |

The backends disagree in one way that dictates the whole FFI design. Wasmer 2.2 can hand out
an owned, instance-lifetime state (a `Weak<WasmerInstance>`). Wasmer 6 cannot: memory and
globals are only reachable through the `StoreMut` inside `FunctionEnvMut`, so the state is a
borrow that lives exactly as long as one host call. **The FFI must target the Wasmer 6
model** — anything designed against 2.2's model cannot be implemented by Wasmer 6.

## Settled decisions

### Reference bundling, and what C takes away

Copy the Rust VM's shape: `VMHooksDispatcher<C>` → `VMHooksHandler<C>` →
`TxVMHooksContext<S>`, which bundles the VM's `TxContextRef` with the executor's
`InstanceState`. `S` is generic over ownership, so the same context type serves an owned
state (2.2) and a `&mut` borrow (Wasmer 6).

Four guarantees the borrow checker gives for free need explicit replacements over C:

1. **Lifetime** — a `u64` call token in the context, bumped on hook entry and invalidated on
   exit, plus nulling the state pointer. Every state function validates it. A stale handle
   becomes a clean error instead of a use-after-free.
2. **Exclusivity** — the context must be **stack-allocated per hook invocation**, never a
   per-executor or thread-local slot. Then nesting (`executeOnDestContext` running a
   contract from inside a hook) is safe by construction.
3. **Thread affinity** — the Wasmer 6 state borrows the host-call frame. cgo keeps the
   goroutine on its OS thread for the call, but a goroutine spawned inside a hook must not
   touch the handle. State it as an invariant in the header.
4. **No panics** — a panic across `extern "C"` aborts the node. Every entry point needs the
   `capi_safe_unwind` treatment.

### The hook ABI: a struct return, verified

A cgo `//export` function **can** return a C struct by value. Verified on darwin/arm64 with
Go 1.23 by building a `c-archive` and calling it from C: 8-, 12- and 16-byte layouts all
return correct values. One 16-byte struct covers every hook shape and is returned in
registers under both SysV AMD64 (RAX:RDX) and AAPCS64 (X0:X1):

```c
typedef struct { int64_t value; uint64_t early_exit; } vm_exec_hook_result_t;
#define VM_EXEC_NO_EARLY_EXIT 0xFFFFFFFFFFFFFFFFULL
```

The sentinel is needed because `early_exit == 0` is a real code — the Rust VM uses
`ASYNC_CALL_EARLY_EXIT_CODE = 0`, and the Go VM has the same async-call case. Verified that
code `0` and "no early exit" stay distinguishable.

### The early-exit message does not cross the boundary

A pointer cannot ride along in the struct: `cmd/cgo/doc.go` (go1.23, line 389) says *"A Go
function called by C code may return a Go pointer to pinned memory (which implies that it
may not return a string, slice, channel, and so forth)."* Returning an unpinned Go pointer
appears to work and is not caught even under `GOEXPERIMENT=cgocheck2`, which is exactly why
it must not be relied on.

A buffer is not the answer either: messages are unbounded. `SignalError`
(`vmhost/vmhooks/baseOps.go:236`) loads `messageLength` bytes straight from wasm memory,
metered per byte, capped only by the memory size.

But the Go VM does not need the text to travel. `SignalUserError` already records it via
`Output().SetReturnMessage(message)` **inside the hook**, and what comes back out of the
call is only a sentinel — `vmhost/hostCore/breakpoints.go:26-44` maps a breakpoint to
`ErrSignalError` / `ErrNotEnoughGas` / `ErrExecutionFailed` / `ErrMemoryLimit`. So the early
exit needs to carry the **code only**.

This is a real asymmetry with the Rust VM rather than an oversight: there the message must
travel, because `TxResult::from_error(code, message)` is the only place the text exists —
and it costs nothing, since the hook returns `Err(VMHooksEarlyExit)` in-process with a
`Cow<'static, str>`.

If the text is ever wanted on the Rust side, it does **not** belong on `InstanceState`: that
trait is call-scoped under Wasmer 6, while an early-exit message must outlive the call, and
an early exit is a property of the call rather than of the instance. It belongs in the FFI
context. Preferred shape is **pull**: one extra hook pointer,
`int32_t get_early_exit_message(void* ctx, uint8_t* buf, int32_t buf_len)` returning the
required length, called by Rust only when a code is set. That keeps the happy path clear and
the obligation out of the hook contract, so it can stay unimplemented until something wants
it.

### Rust does the memory work and the validation

Bounds checking and the exact error taxonomy live in Rust, not Go. Go's
`MemLoadFromMemory`/`MemStoreToMemory` helpers are bypassed for the new path.

### The Go VM's relationship with the hooks changes

Today there is exactly one `VMHooksImpl` for the whole host
(`vmhost/hostCore/host.go:210`), holding only `host`, with "which instance" answered
dynamically by the instance-tracker stack. That is the mirror image of the Rust design and
is what flips.

## Open decision gates

| # | Decision | Why it blocks |
| --- | --- | --- |
| **G1** | Do the hooks hold the state, or take it as a parameter? | The C ABI passes the context as the first argument of every hook — that *is* the parameter form. If the Rust trait keeps the state bundled, the generator must emit and reconcile two shapes. Deciding after the ABI is frozen turns a refactor into a two-repo migration. The Wasmer 6 builder currently returns `Box<dyn VMHooks>`, so a per-hook allocation also rides on this. **Phase 2 found this already answered by the committed Wasmer 6 code**: `ExperimentalVMHooksBuilder::with_vm_hooks` hands the builder the state and lends the hooks back through a closure, so the state is *bundled* in Rust and *passed as a parameter* in C — the two are compatible, and the generator emits one shape, not two. The allocation question is also already resolved: the builder lends `&mut dyn VMHooks` on its own stack frame. |
| **G2** | Does the new executor keep the Go VM's truncating load? | **Settled in Phase 2: yes, it clamps.** `vm_exec2_state_mem_load` writes into the caller's buffer and returns `min(requested, available)`, so the truncation is visible in the return value. Chosen to preserve behaviour contracts have already observed; the strict load stays available on the trait for a caller that wants it. The rule itself moved into `clamped_load_len` in `vm-executor/src/new_traits/memory.rs`, so the FFI and `InstanceState::memory_load_clamped` cannot drift apart. |
| **G4** | Which `OpcodeVersion` does each host declare, and does the config stay mutable? | Decides whether `OpcodeConfig` is `Arc<Mutex<_>>` (2.2 today) or plain (Wasmer 6 today), which decides the shape of the single config struct. **Phase 2 took the immutable branch for the new C API**: the version and the cost table arrive in `vm_exec2_executor_config_t` at `vm_exec2_new_executor` and are never mutated, so there is no `Arc<Mutex<_>>` on this path and no `set_opcode_config`. The gate stays open for Phase 1.6, which has to reconcile the 2.2 backend's `Arc<Mutex<_>>`. |

## Phase 1 — Close the trait gaps

| # | Item | Status |
| --- | --- | --- |
| 1 | `Instance::get_imported_function_names()`, replacing per-name `has_imported_function`; needed by the Go contract validator (`vmhost/contexts/runtime.go:741,750,759`), which is consensus-relevant | **done** |
| 2 | `InstanceState`: `memory_length`, `memory_grow`, strict load/store, plus `memory_load_clamped` for the Go VM's truncating load; shared `MemoryError` + `checked_load_range`/`checked_store_range` in `vm-executor/src/new_traits/memory.rs` | **done** |
| 3 | Bounds-check `WasmerInstance::memory_load`/`memory_store`, which sliced with an unchecked, overflow-prone `mem_ptr + mem_length` | **done** |
| 4 | Remove every panic from `Result`-returning paths: instance init in both executors (needs `rc_new_cyclic_fallible` for the 2.2 backend), `new_instance_from_cache`, `reset`, the breakpoint read in `call`, the prod state setters, `get_memory_view` | **done** |
| 5 | `BreakpointValue` discriminants aligned to the wire values 4/5 so `as_u64` and `TryFrom<u64>` are inverses | **done** (separate commit) |
| 6 | Converge the two `RuntimeRef` traits into one config plus one hooks-provider trait, dropping the `Mutex` | gated on **G1**, **G4** |
| 7 | Apply G1: if the state becomes a hook parameter, the builder trait disappears and `VMHooksWrapper` holds one `Box<dyn VMHooks>` per instance | gated on **G1** |

The bounds rules are deliberately shared by all backends — they are observable by contracts,
so there is one implementation, not one per backend. Ten tests pin the edge cases, including
two asymmetries inherited from Go: a zero length returns empty *before* the offset is
validated, and a store past the end reports `BadBoundsUpper` where a load reports
`BadBounds`.

## Phase 2 — New FFI crate (`c-api-new`, alongside the existing one) — **done**

Built fresh so the wasmer2 path keeps shipping. 16 files in `c-api-new/`, 34 tests.

**Exported surface** (18 functions, all in `c-api-new/libvmexeccapi_new.h`):

- *Lifecycle*: `vm_exec2_new_executor`, `vm_exec2_new_instance`, the two `_destroy`s. One
  config struct per level, passed once; no post-construction setters.
- *Outer instance ops*: `vm_exec2_instance_call` filling a `vm_exec2_call_result_t`
  out-struct, `_get_points_used`, `_check_signatures`, `_has_function`, and the two-step
  exported- and imported-names pairs.
- *State ops*, all taking a hook context: `_mem_load`, `_mem_store`, `_mem_length`,
  `_mem_grow`, `_get_points_used`, `_set_points_used`.

### What was decided while building it

**Symbols are prefixed `vm_exec2_`.** The names in the original sketch —
`vm_exec_new_executor`, `vm_exec_new_instance` — are already exported by the legacy `c-api`,
so a binary linking both libraries would have had duplicate symbols. Verified that the two
`.dylib`s now share no `vm_*` symbol, which is what lets the Go VM hold both executors at
once for the differential run in Phase 5. Types keep their generated names
(`vm_exec_opcode_cost_t`), since only `#[no_mangle]` functions collide; the two headers must
not be included in the same translation unit, which Phase 4's separate `wasmer3/` package
satisfies.

**Errors travel through a caller-provided buffer.** `capi_status` holds the status enum —
`VM_EXEC2_OK = 0` and every failure negative, so the counting functions share the code space
— plus a truncating, NUL-terminating writer that keeps the buffer valid UTF-8. The
`thread_local!` last-error singleton did not come along, as planned. A null pointer argument
returns `VM_EXEC2_NULL_POINTER` rather than a message, because the null pointer may be the
error buffer itself.

**The executor does not wrap a `dyn Executor`, and that is a finding for Phase 1.6.** The new
`Executor::new_instance` takes only the wasm bytes and the compilation options, with the hooks
coming from the executor's own runtime reference. But this API needs one Go hooks object *per
instance* — that is where the Go side keeps the per-instance early-exit slot that Phase 3's
shim reads — and there is nowhere in that signature to put it. So `vm_exec2_new_instance`
takes `go_vm_hooks` and builds the instance directly against the Wasmer 6 backend. **Phase
1.6's converged hooks-provider trait should take a per-instance host handle**, otherwise the
`Executor` trait cannot serve the C API on either backend.

**The hook context is pooled, not stack-allocated.** The original sketch called for a
stack-allocated context per invocation. That is unsafe as specified: a Go side holding the
pointer after the hook returns would be reading a reused stack frame, so even *reading the
token in order to reject it* would be undefined behaviour. Contexts are therefore boxed
individually and kept in a per-instance stack for the instance's lifetime — one allocation per
nesting level, reused across calls — which keeps the exclusivity property the stack gave and
makes the rejection path well-defined. The token check is `ctx.token == pool.active_token`,
so a context belonging to an outer call is rejected while a nested call is running and valid
again afterwards.

### What is deliberately not here

**The hook dispatch.** The typed function-pointer table and the 277-method `VMHooks`
implementation that calls into it are generated code, and belong to Phase 3 as originally
scoped. Everything they need is in place: the result and context ABI, the state operations,
and `CapiVMHooksBuilder::with_hook_context_raw` as the seam. Until then the builder yields
`VMHooksDefault` behind a single marked `TODO(Phase 3)`, and the table is copied as opaque
pointer-sized words (`CapiHookTable`) so the ownership story does not change when the type
does. The copy-rather-than-borrow decision is already made, matching the legacy API.

**CI and release wiring.** `make capi2`, `capi2-linux-amd64`, `capi2-linux-arm`,
`capi2-osx-amd64` and `capi2-osx-arm` produce `libvmexeccapi2*` artifacts with the right
soname / install name. They are not in `ci-local` or the release workflows, so the published
assets do not grow a library nothing links against yet; wire them up when Phase 4 starts
consuming them.

### Testing

The planned "mock hook-pointer table" test became something stronger: a mock hooks *provider*
that drives the real state FFI from inside a real host call, which is the only place a context
is live. `c-api-new/tests/ffi_boundary.rs` covers the lifecycle and the outer ops through the
exported functions, and from inside a hook asserts the memory and gas operations, including
the G2 edge cases (clamped over-read, offset at and past the end, negative offset, negative
length, and the fact that a store is never truncated). It also delivers two of Phase 5's
checks early: a context used after its call is refused with `VM_EXEC2_STALE_CONTEXT`, and so
is an outer context while a nested call is running.

## Phase 3 — Generator

Lands before the Go adapter, since it drives both repos. New writers, modelled on the
existing `eiGenWriteRustCapiVMHooks*.go` and `eiGenWriteCgo.go`:

- the new hook-pointer struct (context first arg, `vm_exec_hook_result_t` return);
- the new Rust capi `VMHooks` impl (call the pointer, compare against the sentinel, return
  `Err`);
- the new Go `//export` shims;
- `eiGenMain.go`: register the writers and the copy targets.

**The 282 Go hook signatures in `executor/vmHooks.go` do not change.** The shim reads an
`earlyExit` field off the hooks object after the call and packs it into the result struct.
That works only if there is one hooks object per live instance, so it depends on the Phase 4
provider change rather than on touching the interface.

## Phase 4 — Go VM (new `wasmer3/` package, parallel to `wasmer2/`)

Both executors coexist behind `ExecutorAbstractFactory`, which is what makes differential
testing possible.

1. `executor.InstanceState` interface — `MemLoad`, `MemStore`, `MemLength`, `MemGrow`,
   `GetPointsUsed`, `SetPointsUsed` — implemented by a struct wrapping the `ctx` pointer.
2. `executor.VMHooks` becomes a provider: `CreateVMHooks(state InstanceState) VMHooks`,
   mirroring the Rust builder. Replaces the single host-wide `VMHooksImpl`.
3. `VMHooksImpl` gains `state` and `earlyExit`; its `MemLoad`/`MemStore` stop going through
   `host.Runtime().GetInstance()`. This is the change that decouples the hooks from the
   instance tracker.
4. `FailExecution` / `SignalUserError` record an early exit instead of setting a breakpoint,
   and keep burning gas as they do now.
5. `handleBreakpointIfAny` maps `InstanceCallResult` instead of a wasmer global, producing
   the same sentinels.
6. `MemLoadFromMemory` / `MemStoreToMemory` are bypassed for wasmer3; kept for wasmer2.
7. The validator switches to imported-function *names* rather than per-name queries.
8. The executor factory passes one config at creation; `SetOpcodeCosts` as a
   post-construction mutator goes away. Note Go is already stale here — `wasmer2/bridge2.go`
   calls `vm_exec_set_opcode_costs` against a header that now exports
   `vm_exec_set_opcode_config`.

Only the memory and gas paths move to the state handle. `IsFunctionImported`,
`ValidateFunctionArities`, `Reset` and `Cache` stay on the outer `Instance`, and keeping that
split crisp is what makes the Go side match the Rust trait split rather than just wrap it.

## Phase 5 — Bring-up and validation

- Unit tests per Phase 1 item, plus the FFI boundary test from Phase 2.
- Targeted Go tests for the G2 memory edge cases: the clamped over-read, negative length, low
  and high store bounds. The three current implementations disagree, so these come before
  anything else is trusted. The Rust side of these is already covered by
  `c-api-new/tests/ffi_boundary.rs`; what remains is asserting the Go wrapper agrees.
- **Differential run**: the scenario suite under both factories, asserting identical
  `VMOutput` (return code, message, logs, gas used). This is the acceptance criterion.
- Re-entrancy: a contract calling `executeOnDestContext` from inside a hook, confirming
  nested contexts do not clobber each other. **Covered at the Rust level in Phase 2**; what
  remains is driving it from a real contract through Go.
- Stale-handle test: keep a `ctx` past the hook return and confirm the token check rejects it
  rather than faulting. **Done in Phase 2** (`state_ops`).

## Phase 6 — Deferred

- `reset()` on the Wasmer 6 instance — prerequisite for Go's warm-instance tracker.
- `new_instance_from_cache` and the `cache` path.
- Only then wire warm instances into wasmer3.

## Cross-repo note

`mx-sdk-rs` pins `multiversx-chain-vm-executor* = "=0.5.1"` from crates.io, so it picks up
none of this until a version bump. At that bump, three accumulated breaks need resolving
together:

- `ExperimentalExecutorRuntimeRef::opcode_cost` → `opcode_config` (carries the G4 decision);
- `VMHooksDispatcher` is missing five newer hooks (`activate_unsafe_mode`,
  `deactivate_unsafe_mode`, `managed_get_num_errors`, `managed_get_error_with_index`,
  `managed_get_last_error`) — the `VMHooks` trait has 282 required methods and no defaults,
  so every addition is a hard break;
- `WasmerProdInstanceState::set_breakpoint_value_legacy` / `set_early_exit` now return
  `Result`, so `InstanceStateSetEarlyExit` in `chain/wasmer-prod` must handle them.

## Critical path

Originally **G1 → Phase 1.6/1.7 → Phase 2 → Phase 3 → Phase 4**. In the event Phase 2 ran
ahead of Phase 1.6/1.7: G1 turned out to be already answered by the committed Wasmer 6 code,
and the C ABI is parameter-form regardless, so the trait convergence was not a prerequisite
after all — it is now better informed by Phase 2, which found the per-instance hooks argument
the converged trait needs.

Remaining path: **Phase 3 → Phase 4 → Phase 5**, with Phase 1.6/1.7 landing whenever G4 is
settled across both backends. Phase 3 is now the only thing between here and a Go consumer.
