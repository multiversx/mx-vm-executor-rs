# Integrating the new executor traits into the Go VM

Working plan for exposing `vm-executor/src/new_traits` over a C FFI and consuming it from
the Go VM. Status as of 2026-10-02: Phase 1 items 1-5, Phase 2, Phase 3 and Phase 4a are
implemented; Phase 4b is in progress.

## Why

The repo was built around Wasmer 2.2, hidden behind the legacy interfaces in
`vm-executor`. Those interfaces cannot express Wasmer 6's ownership model for VM hooks, so a
second set of traits was added in `vm-executor/src/new_traits`, currently driven only by the
Rust VM (`mx-sdk-rs/chain/vm`). The Go VM still speaks the legacy C API. This plan covers
the new FFI crate, the generator work, and the Go-side changes.

Design principles: configure the executor directly and all at once; prefer pure functions;
one config object rather than a sequence of setters.

## Where things live

Four integrations, because two of them are the *same backend* reached two ways: wasmer 2.2
through the legacy interfaces and wasmer 2.2 through the new ones. Both are kept until the
migration is shown to be correct, which is what the hook-sequence differential is for.

| Backend | Interfaces | Crate | Symbols | Library | Go package |
| --- | --- | --- | --- | --- | --- |
| wasmer 2.2 | legacy | `c-api-legacy` | `vm_exec_*` | `libvmexeccapi` | `wasmer2legacy` |
| wasmer 2.2 | new | `c-api-wasmer2` *(planned)* | `vm_exec_w2_*` | `libvmexeccapi_w2` | `wasmer2` *(planned)* |
| wasmer 6 | new | `c-api-wasmer6` | `vm_exec_w6_*` | `libvmexeccapi_w6` | `wasmer6` |
| — | shared | `c-api-common` *(planned)* | `vm_exec_*` | header only | `wasmercommon` *(planned)* |

The numbers name the **backend**, not the interface generation. The old `vm_exec2_*` prefix
meant "second interface set", which stopped working the moment a second backend joined that
set; it is retired rather than recycled, so no prefix ever changes meaning. The shared types
go into one `libvmexeccapi_common.h` that both backend headers include, which is what lets a
Go package include both and a binary link both without redefinition — and what gives
`wasmercommon` a single set of Go types to wrap.

One constraint that falls out: a Go package must not include the legacy header together with a
new one. They both declare `vm_exec_compilation_options_t` and `vm_exec_opcode_cost_t`, and the
compilation options differ in layout (seven fields against four). No package does, and the
problem dies with `c-api-legacy`.

Cgo `//export` shim names are C symbols too, so they also have to be unique per package:
`w2_*` for the legacy shims and `w6_*` for wasmer 6. The new wasmer 2.2 package will need a
third prefix rather than reusing `w2_`.

| Piece | Path |
| --- | --- |
| Interfaces, legacy + new | `vm-executor/src/` and `vm-executor/src/new_traits/` |
| Wasmer 2.2 backend (legacy native, new traits via a shim) | `vm-executor-wasmer/` |
| Wasmer 6 backend (new traits native) | `vm-executor-experimental/` |
| C APIs | see the table above |
| Code generator (drives **both** repos) | `mx-chain-vm-go/vmhost/vmhooks/generate/` |
| Go VM | `mx-chain-vm-go/`, cgo bridges in `wasmer2legacy/` and `wasmer6/` |
| Rust VM, Wasmer 6 consumer | `mx-sdk-rs/chain/vm/src/executor_impl/we_executor.rs` |
| Rust VM, Wasmer 2.2 consumer | `mx-sdk-rs/chain/wasmer-prod/src/wasmer_prod.rs` |

## How the two interfaces differ

| | Legacy | New |
| --- | --- | --- |
| `Executor` | 4 methods, 2 of them post-construction setters | 2 instantiation methods; everything else at construction |
| `CompilationOptions` | 7 fields | 4; `gas_limit` moved to `call`, metering and breakpoints always on |
| Instance | one 17-method trait | `Instance` (8, outside view) + `InstanceState` (8, inside view, for hooks) |
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
| **G4** | Which `OpcodeVersion` does each host declare, and does the config stay mutable? | **Resolved in Phase 4b: immutable, with the executor rebuilt on change.** Phase 2 had already taken the immutable branch for the new C API — version and costs arrive in `vm_exec2_executor_config_t`, with no `set_opcode_config`. The obstacle was `GasScheduleChange` (`vmhost/hostCore/host.go:375`), which re-applies both to the *live* executor at an epoch. It is reconcilable because that function already calls `ClearWarmInstanceCache()`, so no instance survives the change: recreating the executor is behaviour-preserving, and is what lets `SetOpcodeConfig` leave the `Executor` interface. Phase 1.6 still has to reconcile the 2.2 backend's `Arc<Mutex<_>>` if that path is ever built. |
| **G5** | Does one C library carry both backends, selected by a flag? | **Answered: no — one library per backend, link the one you need.** That is already the pattern: a node links four wasmer runtimes today (v1.2/v1.3/v1.4 share one byte-identical `libwasmer`, v1.5 brings `libvmexeccapi`) with zero shared symbols between them. Bundling would make the 32 duplicate `wasmer_vm_*` libcalls a *static* collision the linker silently collapses, breaking one backend's `vmctx` layout; two libraries make it a *dynamic* one that `-Bsymbolic-functions` fixes. Duplication is near zero: one Rust crate with a backend feature, one Go bridge with a build tag. What remains is **verifying `-Bsymbolic-functions` on Linux, before anything links both** — see "Serving both backends" below. |


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
pointer-sized words (`CapiVMHookTable`) so the ownership story does not change when the type
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

## Phase 3 — Generator — **done**

Three new writers in `mx-chain-vm-go/vmhost/vmhooks/generate`, registered in `eiGenMain.go`
with their copy targets. All 277 hooks, generated into both repos.

| Writer | Output |
| --- | --- |
| `WriteRustCapi2VMHooksPointers` | `c-api-new/src/capi_vm_hook_pointers.rs` |
| `WriteRustCapi2VMHooks` | `c-api-new/src/capi_vm_hooks.rs` |
| `WriteWasmer3Cgo` | `mx-chain-vm-go/wasmer3/wasmer3ImportsCgo.go` |

`capi_opcode_cost.rs` also gained a second copy target, so `c-api-new` no longer carries a
hand-copied duplicate of it.

### The `earlyExit` field is not needed after all

The original sketch had the shim reading an `earlyExit` field off the hooks object after the
call, and noted that this "depends on the Phase 4 provider change". That was written before
the Go hook migration landed. Every hook now **returns** its error, so the shim converts it
inline:

```go
//export w3_getGasLeft
func w3_getGasLeft(ctx *C.vm_exec2_hook_context_t) C.vm_exec2_hook_result_t {
	vmHooks := getVMHooksFromHookContext(ctx)
	result, err := vmHooks.GetGasLeft()
	if err != nil {
		return hookEarlyExit(vmHooks, err)
	}
	return hookOk(int64(result))
}
```

No field, no ordering hazard, and no dependency on Phase 4. The 277 Go hook signatures in
`executor/vmHooks.go` still do not change.

### Converting the error to a code

`executor/earlyExit.go` owns the `EarlyExit` struct — it is what crosses the boundary, so it
belongs in the package the VM and the backends share rather than in the VM. Alongside it:

- `ExitExecutionFailed`, the one definition of what a plain error means (execution failed,
  burn the remaining gas). Both the VM, which applies the side effects, and the executor,
  which only reports a code, go through it.
- `EarlyExitCodeOf`, a pure function finding the code with `errors.As`, so it survives the
  `fmt.Errorf("%w")` wrapping the hook helpers do.

`vmhost` keeps the two constructors that deviate from that default, `ExitUserError` and
`ExitOutOfGas`, because they carry VM sentinel errors. `vmhooks.resolveEarlyExit` keeps its
single call site and delegates its default branch to `ExitExecutionFailed`.

An earlier iteration had `EarlyExit` stay in `vmhost` with an `EarlyExitError` interface in
`executor` to reach it. That was dropped: the interface had exactly one implementor and no
prospect of a second, and it is not what kept `wasmer3` from importing `vmhost` — a struct in
`executor` preserves that property just as well, while letting the default live once instead
of twice.

The wire codes are `vmcommon.ReturnCode` values, matching the Rust VM's `ReturnCode::as_u64`.
Nothing in the failure path produces `vmcommon.Ok`, which is what keeps code 0 free to mean
"legacy async call", as `ASYNC_CALL_EARLY_EXIT_CODE` does on the Rust side.

`hookEarlyExit` calls `HandleHookError` for the side effects and `EarlyExitCodeOf` for the
code. Under this executor the breakpoint `HandleHookError` still sets is redundant — the early
exit already stops the instance — and it goes away with `PLAN-go-breakpoint-removal.md`.

### The generated shims check their own ABI

cgo types the hook table's fields as `*[0]byte`, so the Go assignment in
`populateCgoFunctionPointers` would accept a function of any shape, and a mismatch would only
show up as a corrupted stack at the first call. The generated file therefore emits a C
initializer assigning every shim to its field — the one place both types are visible at once:

```c
// static const vm_exec2_vm_hook_c_func_pointers w3_signature_check __attribute__((unused)) = {
//     .get_gas_left_func_ptr = w3_getGasLeft,
//     ...
```

Verified by deliberately widening one shim's argument and confirming the build fails on
exactly that field.

### Rust side wired up

`CapiVMHookTable` now holds a copy of the generated table rather than opaque words, and
`vm_exec2_new_executor` takes it typed, so the Phase 2 placeholder parameter pair
`(hook_table, hook_table_len)` is gone. `CapiVMHooksBuilder` builds a `CapiVMHooks` around the
activated context instead of yielding `VMHooksDefault`; the Phase 2 `TODO(Phase 3)` is closed.

### Testing

`hook_dispatch` in `c-api-new/tests/ffi_boundary.rs` runs a contract that calls two hooks
through a mock table, covering the whole round trip in one test: wasm into Rust, out through a
C function pointer, back into Rust through the state functions to read the contract's memory,
and — for the hook that asks to stop — out again as `VM_EXEC2_CALL_EARLY_EXIT` carrying the
code. It also pins the thing the whole ABI rests on: the sentinel keeps "returned normally"
distinct from early-exit code 0.

On the Go side, the package compiling *is* the check that a cgo `//export` function can return
a C struct by value — now across all 277 shims rather than the original three-layout spike.

## Serving both backends

The two interface sets cannot coexist in one VM: `Instance` lost most of its methods and the
inside view moved to `InstanceState`, so a host either speaks the old set or the new one. The
Rust side has already resolved this the right way round — **the new set is not "Wasmer 6's
interface", it is the intersection**. Wasmer 2.2 satisfies it through
`vm-executor-wasmer/src/new_traits`, and the Rust VM drives both backends through it; the
reverse does not hold, because nothing in the legacy set can express a call-scoped state.

That asymmetry is what makes converging safe, and it is worth being explicit about why. The
hook context (`capi_vm_hook_context`) enforces the *stricter* of the two lifetimes: a state
handle is valid for exactly one host call. Wasmer 6 requires that. Wasmer 2.2 does not — its
`WasmerProdInstanceState` holds a `Weak<WasmerInstance>` and stays valid for the whole
instance — but it satisfies the stricter contract trivially. Writing the Go side against the
tighter contract therefore costs nothing on 2.2 and makes it impossible for the VM to
accidentally depend on a lifetime only 2.2 offers.

### Two libraries, not one — G5 answered "no"

The original target was a single `libvmexeccapi2` carrying both backends, chosen by a field in
`vm_exec2_executor_config_t`. **That is dropped.** What replaces it is one library per backend,
with the node linking the one it needs — which is not a compromise but the pattern this
codebase has always used.

**The established pattern.** A production node already links *four* wasmer runtimes:
`mx-chain-vm-v1_2-go`, `-v1_3-go` and `-v1_4-go` each ship `wasmer/libwasmer_*`, and
`mx-chain-vm-go v1.5` ships `wasmer2/libvmexeccapi*`. The three older copies are
byte-identical (same sha256, same size), and the old `libwasmer` and `libvmexeccapi` share
**zero** symbol names. So the invariant that has quietly held is not "one runtime per process"
— it is **no two *different* runtimes with colliding symbol names**.

**Why one library is the worse option.** Bundling makes the collision a *static* one: both
`wasmer-vm` versions define the same 32 `#[no_mangle]` libcalls, the linker collapses them to
one definition, and the context-taking ones then reinterpret `vmctx` under the wrong layout.
Fixing that means renaming symbols in one version's rlib or patching `wasmer-vm`. Two
libraries make it a *dynamic* one, which is a link flag — see below.

**What the duplication actually costs: almost nothing.** `c-api-new` touches its backend in
**three type names across two of thirteen files** — `ExperimentalInstance` in
`capi_instance.rs`, `ExperimentalInstanceState` and `ExperimentalVMHooksBuilder` in
`capi_vm_hook_context.rs`. Status codes, strings, config, opcode cost, the hook context
machinery and the 908-line generated hooks implementation are all backend-agnostic. So the
shape is **one crate with a Cargo feature selecting a backend alias module, built twice**, not
two crates. The one snag is that the aliases are not drop-in: `ExperimentalInstanceState<'a>`
is a borrow of the host-call frame while `WasmerProdInstanceState` is owned, so the lifetime
shapes have to agree first — which is Phase 1.6's job regardless.

Likewise on the Go side: if both libraries keep the same `vm_exec2_*` ABI, there is **one**
bridge, with a build tag choosing between the `LDFLAGS` lines, and the 3,631-line generated
cgo file stays single-copy. Two Go bridges would only be needed if the libraries were
deliberately given different symbol prefixes, and there is no reason to.

### The symbol problem does not go away, and it is on the Phase 5 path

Two libraries does *not* dodge the collision, because the deployment that matters links both.
The migration shape here is the v1.2–v1.4 one: ship the new VM module version alongside the
old, so a node syncing from genesis can re-execute historical blocks under the backend that
produced them. At that point the node links `libvmexeccapi` (wasmer 2.2) **and**
`libvmexeccapi2` (wasmer 6) — and those two shipped libraries have **32 overlapping exported
symbol names, every one of them `wasmer_vm_*`** (measured directly:
`llvm-nm --dynamic` on the shipped `.so`, `nm -gU` on the new dylib). Under ELF's flat
namespace either library's libcall references can bind to the other's definition. macOS's
two-level namespace is why `wasmer3/dualbackend/` passes locally and tells us nothing about
Linux.

The candidate fix is **`-Wl,-Bsymbolic-functions` on both libraries**: it binds each library's
intra-library references at link time, so each backend reaches its own libcalls regardless of
what else is loaded. It composes with the version script rustc already passes, unlike an
export list (see above). **It needs a Linux verification, and that verification belongs before
anything links both** — which means before the Phase 5 differential harness, not after.

### The second new-FFI library may not be needed at all

Worth stating plainly, because it removes work from the critical path. The reason to want
wasmer 2.2 behind the *new* FFI is to delete the legacy C API. It is **not** needed for the
differential run: after Phase 4b, `wasmer2` presents the new Go interfaces over the legacy
`libvmexeccapi`, because 2.2 hands out an instance-lifetime state and a Go-side
`InstanceState` for it is a thin wrapper over the existing instance — which is what Phase 4b
item 1 already says. So the differential run is `wasmer2` against `wasmer3`, both speaking the
new Go interfaces, with no second Rust artifact.

That makes **Phase 1.6 and the `VMHooksLegacyAdapter` activation trick cleanup rather than
migration**: worth doing when the legacy interface set is retired, not before wasmer 6 is live.
The `-Bsymbolic-functions` work is what takes its place on the critical path.

### What the wasmer 2.2 path needs

`WasmerProdExecutor` already implements the new `Executor` trait, so the gap is in the hooks,
not the instance:

- `WasmerProdRuntimeRef::vm_hooks` returns `Box<dyn VMHooksLegacy>` and `opcode_config()`
  returns `Arc<Mutex<OpcodeConfig>>`, where the Wasmer 6 side returns a hooks *builder* and a
  plain `Arc`. Converging the two (**Phase 1.6**) is therefore a prerequisite for this path,
  not an optional tidy-up.
- The generated `CapiVMHooks` is new-style and borrows a per-call context. Wasmer 2.2 asks for
  the hooks once per instance. Rather than generating a second 277-method implementation, the
  capi should reuse the one it has behind `VMHooksLegacyAdapter`: that adapter already wraps
  every hook call, so a capi-specific `VMHooksSetEarlyExit` can **activate a hook context
  around each call** and build `CapiVMHooks` there. One generated implementation, two
  activation paths, and the Go side cannot tell which backend it is talking to.
- `set_early_exit` on that path must do what `mx-sdk-rs/chain/wasmer-prod/src/wasmer_prod.rs`
  does: store the exit *and* trip the breakpoint global, because under 2.2 the global is still
  the mechanical halt. See `PLAN-go-breakpoint-removal.md` §3.

## Phase 4 — Go VM

Split in two, because the two halves fail for different reasons and only the first one is
cheap to undo.

### Phase 4a — Make the FFI work, with the VM kept out of it — **done**

A `wasmer3/` package that links the library and drives it directly, with a Go-side test
supplying a minimal hand-written `executor.VMHooks`. It does **not** implement
`executor.Executor` or `executor.Instance`, so a wrong turn costs a package rather than a
refactor.

| File | What it is |
| --- | --- |
| `wasmer3.go` | Package doc and logger |
| `bridge3.go` | cgo linkage and a thin wrapper per C entry point, all 18 of them |
| `error3.go` | The caller-provided error buffer, and status → Go error |
| `wasmer3Executor.go` | `ExecutorConfig`, `CompilationOptions`, executor and instance creation |
| `wasmer3Instance.go` | The outside view: call, gas, introspection, `CallResult` |
| `wasmer3InstanceState.go` | The inside view: the six state operations, over a hook context |
| `vmHooksProvider.go` | `VMHooksProvider`, and the per-instance handle it is reached through |
| `wasmer3ImportsCgoHelper.go` | What every generated shim calls on entry and on failure |
| *(no `opcodeCost.go`)* | The cost table moved to `executor`; see below |
| `bridge3_test.go` | 20 tests (23 with subtests), the Phase 4a acceptance |
| `dualbackend/` | The G5 experiment, behind a build tag |

What the five questions turned out to be:

1. **Linking.** The same `${SRCDIR}` rpath shape as `wasmer2/bridge2.go`, against
   `libvmexeccapi2[_arm]`. The darwin/arm64 library is built and in place; the other three
   platforms need a CI build before they link.
2. **The struct-return ABI at runtime — confirmed.** `TestHookResultShapes` drives an `i64`,
   an `i32` and a `void` hook from wasm, and the contract itself compares the value it got
   back against what the Go hook promised, signalling an error if they differ. Checked for
   non-vacuity by flipping one bit of the hook's return: the test fails, as an early exit with
   the contract's own error code. Until this ran, the 16-byte result struct was proven in two
   halves and never end to end.
3. **The `go_vm_hooks` pointer — `runtime/cgo.Handle`,** which meant **changing the C type
   from `void *` to `uintptr_t`.** The legacy path stores a Go pointer in C memory, which cgo
   forbids and which works by luck; a handle is an integer index into a Go-side table, which
   is what the rule leaves available. The type change was necessary rather than cosmetic:
   with a `void *` field, Go has to convert the handle back with `unsafe.Pointer(uintptr(...))`,
   which is exactly the pattern `go vet`'s `unsafeptr` check rejects. The field is also more
   honestly typed now, since Rust only stores the word and hands it back. The whole package
   passes `go vet` and `GOEXPERIMENT=cgocheck2`, the strictest pointer checking available —
   verified to be live by making it catch a deliberate violation.
4. **State operations from inside a hook, and stale-context rejection.** Both work from Go:
   a hook writes wasm memory through `MemStore` and reads it through `MemLoad`, with the
   contract verifying the bytes itself, and reads gas and memory size through the same
   context. `TestStaleContextIsRejected` keeps the state past the hook's return and confirms
   all six operations come back as `ErrStaleContext` rather than following a dead borrow.
5. **The signal-handler experiment.** Done, in `dualbackend/`; it passed, and it turned up a
   different G5 blocker. Both recorded under "Serving both backends".

Two risks the phase closed on its own account:

- **Nesting across the boundary works** (`TestHookReentersTheExecutor`). A hook runs a whole
  other instance, which runs hooks of its own, and the outer hook's state is still usable
  afterwards and unchanged. That is the property the per-call hooks decision below rests on,
  so it is proven here rather than deferred to Phase 5.
- **Re-entering the *same* instance is now an error rather than undefined behaviour.**
  `ExperimentalInstance::call` takes `&mut self`, so a second call placed while the first is in
  flight would alias a mutable borrow across C, with nothing to catch it. `Wasmer3Instance`
  carries an `inCall` flag, so the hook gets `ErrInstanceReentered` and it travels out as an
  ordinary early exit. The VM never does this — the instance tracker gives each stack level its
  own instance — but it was available to do, and this is the same category of replacement as
  the stale-context token: a guarantee the borrow checker used to give, reinstated by hand.

Two things were decided here that the plan had left open:

- **The hooks are built per call, not per instance** (`VMHooksProvider.VMHooksForCall`),
  mirroring the Rust side exactly: `ExperimentalVMHooksBuilder::with_vm_hooks` builds its
  hooks around the state for the duration of one call and lends them out.

  The alternative — one hooks object per instance with a mutable "current state" slot — is
  *workable*, and it is worth being straight about why, because the obvious objection to it
  turns out to be wrong. A nested call reaches a *different* instance and therefore a
  different slot, so the outer hook's slot survives, and re-entering the same instance is now
  refused outright. It would not be clobbered.

  The reasons to prefer the parameter anyway are smaller but real. It needs no slot, so there
  is no question of when to clear one, and no window in which the hooks object holds a context
  belonging to a call that has ended. It makes the state a parameter, which is the shape the C
  ABI already has — G1's answer — so the generator emits one shape rather than reconciling
  two. And it does not lean on the re-entry guard for a property that construction gives for
  free. The cost is one small allocation per hook call, and no generator change at all: the
  generated shim still calls `getVMHooksFromHookContext(ctx)` and nothing else.
- **Who holds the error buffer.** Outer calls allocate one per call, which is free at their
  frequency. State operations share one buffer per instance, which is sound and not merely
  convenient: a state operation is a single cgo call into Rust that touches memory and
  returns, never calling back into Go, so the buffer's live range contains no re-entry. A
  buffer per operation would be an allocation on every memory access a contract makes.

**The opcode cost table became shared rather than per-backend.** It started as a second
generated copy for `wasmer3`, mirroring `wasmer2/opcodeCost.go`; that was then recognised as
duplication with no cause. The opcodes a cost table covers are a property of the gas schedule
and of the metering middleware, not of the runtime underneath, which is exactly why both C
headers already declared an identical `vm_exec_opcode_cost_t`. So:

- `executor/opcodeCost.go` now holds `OpcodeCost` and `ExtractOpcodeCost`, generated once;
  `wasmer2/opcodeCost.go` and `wasmer3/opcodeCost.go` are deleted.
- The two generator writers merged into one `WriteExecutorOpcodeCost`. The pre-existing writer
  for the *full* WASM set became `WriteExecutorWASMOpcodeCost`, so each writer is now named
  after the type it emits (`OpcodeCost` is the metered subset; `WASMOpcodeCost` is the whole
  set as the gas schedule supplies it).
- `ExtractOpcodeCost` is a package-level pure function rather than wasmer2's former method on
  the executor, since nothing in the conversion depends on an executor.
- **Each backend keeps its own size tripwire**, and they now carry more weight than before: one
  shared Go struct has to match *two* independently generated C headers, and the backends
  reinterpret one as the other to cross the boundary. Both assertions were confirmed to fire by
  adding a field and watching each package fail to compile. wasmer2 previously had no such
  check at all.

Verified by the full `vmhost/hosttest` contract-execution suite, which exercises wasmer2's
metering through the moved struct.

Not done here, deliberately: the `reset`/cache entry points (Phase 6), and the three
non-darwin platform libraries. **The missing libraries have a CI consequence**: `wasmer3` now
links `libvmexeccapi2`, so `go test ./...` fails at the link step on any platform whose library
is absent. `go build ./...` is unaffected, because a non-main package is never linked. Either
the four libraries land before the package is in the default test path, or `wasmer3` goes
behind a build tag until they do.

### Phase 4b — Converge the Go executor interfaces — **in progress**

The Go side still speaks the legacy interface set, and this is where it stops. It is the Go
mirror of the split the Rust side already made, and it is the bulk of Phase 4.

The asymmetry to close: `executor.Instance` has 24 methods, against 8 on the new Rust
`Instance` plus the 8 of an `InstanceState` the Go side does not have at all. A census of the
call sites confirmed the shape is wide but shallow — almost all consumption is
`vmhost/contexts/runtime.go` and `instanceTracker.go`, and only three implementors matter
(`wasmer2.Wasmer2Instance`, `mock.InstanceMock`, `executor/wrapper.WrapperInstance`).

#### Done: `executor.InstanceState` and the hooks provider

The backbone, and it turned out to be far smaller than the method counts suggest. Two
measurements decided the shape:

- **`GetInstance()` has 8 call sites in the whole repo**, two of which are the memory seam.
- **The `vmhooks` package needs exactly two things from an instance**: `MemLoad` and
  `MemStore`, both funnelled through `VMHooksImpl` in `vmhost/vmhooks/vmHooksImpl.go`. Every
  one of the 277 hook bodies goes through `context.MemLoad` / `context.MemStore` /
  `context.MemLoadMultiple`, so re-pointing two one-line method bodies moves all of them.

So `executor.InstanceState` is **two methods, not six**. Gas accounting and memory growth were
candidates on symmetry with the Rust trait, but their Go callers (`contexts/metering.go`,
`contexts/runtime.go`) sit outside a host call as often as inside one, and `MemLength`,
`MemGrow` and `MemDump` are marked "only called directly in tests" in wasmer2 itself. Inventing
state surface for symmetry would have been surface nothing uses.

`executor.VMHooksProvider` has **two** methods, and the asymmetry is deliberate:

- `VMHooksForCall(state)` — Wasmer 6, which hands each host call a context and nothing longer.
- `VMHooksForInstanceLifetime()` — Wasmer 2.2, whose legacy C API sets the hooks pointer
  **per executor, not per instance** (`vm_exec_executor_set_vm_hooks_ptr`; there is no
  instance-level equivalent, which is also why `SetVMHooksPtr` on the Go interface never had a
  caller). A hook therefore cannot be told which instance called it, so the provider resolves
  that through the host's instance stack — which is what the VM has always done.

This is the one place the Go side cannot mirror Rust, where 2.2's per-instance builder holds
the instance weakly. It is forced by the legacy C API, not chosen, and naming both halves keeps
it visible instead of hidden behind a nil convention.

`VMHooksImpl` now takes its state at construction. `NewVMHooksImpl(host)` keeps its signature
and supplies `hostInstanceState`, so the ten-odd test call sites were untouched;
`NewVMHooksImplForCall(host, state)` is the call-scoped form. Verified by `vmhost/hosttest` and
`vmhost/vmhookstest`, and checked for non-vacuity by breaking `hostInstanceState.MemLoad` and
watching `vmhookstest` fail.

#### Done: the validator's imported-function check

Plan item 5 said the validator should switch to imported-function *names*. The census says
otherwise, and the better answer is the reverse: **the VM keeps asking `IsFunctionImported`,
and the backend answers from a cached set.**

The validator checks a 21-name blocklist (`mapNewCryptoAPI` 6, `mapBarnardOpcodes` 15) against
every contract. The legacy C API can only answer one name per call, so that is 21 cgo calls per
validation; the new header is the only one with `imported_function_names` at all. So
`Wasmer3Instance` reads the list once at creation — two cgo calls — and answers from a map.
No interface change, identical semantics, and the consensus-relevant check stays exactly as
written.

#### Done: wasmer3 is selectable, and the differential harness is running

The most useful thing to have early, because it replaces reasoning about what the new backend
does with observing it. `wasmer3.ExecutorFactory()` implements
`executor.ExecutorAbstractFactory`, and `$VMEXECUTOR=wasmer3` selects it — the same switch that
carried the Wasmer 1 to Wasmer 2 migration, which had decayed to one option.

Serving the legacy 24-method `Instance` before it is shrunk needs an adapter, and it is kept in
`wasmer3/wasmer3InstanceLegacy.go` and `wasmer3ExecutorLegacy.go` so that the split stays
visible and those files can be deleted rather than untangled. Three groups of method: plain
forwarding with an error the legacy signature cannot report; memory and gas, which this
executor can only address from inside a host call; and the things it has no notion of —
breakpoint globals, the per-instance hooks pointer, reset and cache.

The middle group needed one mechanism: `instanceHooks.activeState`, the state of the hook
currently running on the instance, published by the single helper the generated shims already
call. No generated code changed. It is safe without clearing, for the reasons the re-entry guard
established in 4a: a nested call runs on a different instance, re-entering the same one is
refused, and a context used after its call fails the token check.

**Shared, not duplicated.** `functionNames` existed in two copies (wasmer2 and the mock) and a
third was about to be generated; it is now `executor.VMHookNames`, generated once by
`WriteExecutorVMHookNames`. Same argument as the opcode cost table: the hook names are a
property of the executor interface, not of what runs below it.

**One VM change was needed, and it is a latent bug fix.** `runtimeContext.saveCompiledCode`
serialised the module and saved the warm instance behind a shared early return, so an executor
that cannot serialise lost its warm instance too. Invisible on wasmer2, where serialising does
not fail; under wasmer3 it was the single cause of **all 42** `vmhookstest` failures, reported as
`unclosed cold instances remaining`. Split into `saveCompiledCodeToStorage` plus
`saveWarmInstance`; the warm path already degrades correctly when `Reset()` fails.

Where the suites stand under `$VMEXECUTOR=wasmer3`:

| Suite | Result |
| --- | --- |
| `vmhost/vmhookstest` | **passes in full** |
| `vmhost/hostCore` | **passes in full** |
| `vmhost/contexts` | 8 failures, all `TestRuntimeContext_Mem*` — tests that address memory through `Instance` from *outside* a host call |
| `vmhost/hosttest` | 17 failures: 8 async-call, 2 memory reset, the rest gas and bad-contract cases |

Every remaining failure is in a named, already-planned bucket: the breakpoint-as-halt work
(async call is the `BreakpointAsyncCall` producer), `Reset()` and the module cache (Phase 6),
and memory reached through `Instance` from outside a call (item 1 below). None is a surprise and
none needed a new gate.

#### Done: the hook-sequence differential, and it already agrees

`integrationTests/json/scenariosExecutorDiff_test.go` runs a scenario folder under both
backends with the `executorwrapper` logger and compares the traces. The wrapper records every
hook with its arguments and its result, so agreement there is a much stronger statement than
matching `VMOutput`s.

On the `erc20-c` suite — a **9,135-line** trace — the two backends differ on **96 lines**, and
every one of them is instance lifecycle: `Reset`, `Clean`, and the `SetPointsUsed` /
`SetGasLimit` / `SetBreakpointValue` triple that follows a successful warm reuse. **Every VM
hook call and every gas value is identical**, which is the first direct evidence that the two
metering middlewares agree. The difference is entirely `Reset()` returning false, which is
Phase 6.

The test is skipped unless `VMEXECUTOR_DIFF=1`, since it cannot pass until Phase 6 lands, and
`VMEXECUTOR_DIFF_FOLDER` chooses the scenario. `TestCErc20Executors` was also revived to its
original purpose: wasmer2, wasmer2, wasmer3, wasmer3, wasmer2 in one process, which passes and
exercises backend coexistence through the real VM rather than through a trap-handler probe.

#### Still to do

1. Shrink `executor.Instance` to the outside view, pushing the legacy-only methods down into
   wasmer2's own types: `SetBreakpointValue`/`GetBreakpointValue` (the new executor reports
   breakpoints in its call result), `SetVMHooksPtr`/`GetVMHooksPtr` (no VM caller at all),
   `SetGasLimit`, `MemDump`, `HasMemory`.
2. `CallFunction` returns an `InstanceCallResult` rather than a bare `error` — the hard
   prerequisite for `PLAN-go-breakpoint-removal.md`.
3. `MemLoadFromMemory` / `MemStoreToMemory` bypassed for wasmer3, where bounds checking lives
   in Rust, and kept for wasmer2.
4. `SetOpcodeConfig` removal — see the G4 resolution below.
5. ~~`wasmer3` implements `executor.Executor` and `executor.Instance`~~ — **done**, as a
   transitional adapter; it is item 1 that lets the adapter go away.

#### G4 resolved: the config can be immutable, if the executor is rebuilt

The gate asked whether `OpcodeConfig` stays mutable. The new C API took the immutable branch in
Phase 2 — version and costs arrive in `vm_exec2_executor_config_t` and there is no
`set_opcode_config`. The obstacle was whether the Go VM needs to mutate it, and it does:
`vmhost/hostCore/host.go:375`, in `GasScheduleChange`, re-applies the opcode version and costs
to the **live** executor when the gas schedule changes at an epoch.

That is reconcilable without making the config mutable, because `GasScheduleChange` already
does the hard part: it calls `ClearWarmInstanceCache()`, so no instance survives the change
anyway. **Recreating the executor rather than mutating it is therefore behaviour-preserving**,
and it is what lets `SetOpcodeConfig` leave the `Executor` interface. The cost is that
`RuntimeContext` needs a way to replace its executor, which is what item 4 above covers.

#### An open gap worth recording: `SetPointsUsed` from outside a host call

The new FFI exposes `vm_exec2_instance_get_points_used` on the outer instance but **no
`set_points_used`** — writing gas is only reachable through a hook context. The VM does write
it from outside a call, at `vmhost/contexts/runtime.go:311`, where reusing a warm instance
resets its counter to zero.

For now this is harmless, because warm instances are deferred to Phase 6 for wasmer3. It stops
being harmless the moment they are not, and the fix is a Rust-side addition rather than a Go
workaround. Recorded here so Phase 6 does not rediscover it.

## Phase 4c — Two libraries per backend, and the shared layer

Driven by the conclusion that bundling is out (gate G5) and that the two Wasmer 2.2
integrations have to coexist until the new one is trusted.

### Done: the renames, and the prefix scheme

Mechanical but load-bearing, because every later stage inherits the names.

- Rust: `c-api` → `c-api-legacy`, `c-api-new` → `c-api-wasmer6`, with `vm_exec2_*` →
  `vm_exec_w6_*` and `libvmexeccapi_new.h` → `libvmexeccapi_w6.h`. Makefile targets
  `capi2-*` → `capi-w6-*`, artifacts `libvmexeccapi2*` → `libvmexeccapi_w6*`.
- Generator: the `Capi2` writers became `CapiW6`, `WriteWasmer3Cgo` became `WriteWasmer6Cgo`,
  the emitted shim prefix `w3_` became `w6_`, and every copy target moved with its crate. Also
  the legacy cgo writer's emitted package, which is what the first regeneration caught.
- Go: `wasmer3` → `wasmer6` (directory, package, files, `Wasmer3*` → `Wasmer6*`), and
  `wasmer2` → `wasmer2legacy`. The node does not import either package — checked against
  `mx-chain-go`, which reaches the VM only through `vmhost` — so this was internal, 13 files.
- `$VMEXECUTOR` values now name backends unambiguously: `wasmer2legacy` (the default, what
  production runs), `wasmer6`, and `wasmer2` **reserved** for the new Wasmer 2.2 integration so
  the two can never be confused.

Verified by regenerating — which must move nothing — and by running the full suite under both
`wasmer2legacy` and `wasmer6`, plus the dual-backend trap tests and the hook-sequence
differential.

### Still to do

1. **`c-api-common`.** The extraction is small: of the thirteen modules in `c-api-wasmer6`,
   **ten already reference no backend type at all**. Only `capi_vm_hook_context.rs` (10
   references), `capi_instance.rs` (3) and `capi_vm_hook_table.rs` (1) are coupled. So status
   codes, strings, config, opcode cost, the logger, the hook result struct and the generated
   hook tables move as-is; the context and the instance stay behind, parameterised by backend.
2. **`c-api-wasmer2`.** Wasmer 2.2 through the new interfaces. This is what puts **Phase 1.6
   back on the critical path**: `WasmerProdRuntimeRef::vm_hooks` returns
   `Box<dyn VMHooksLegacy>` and `Arc<Mutex<OpcodeConfig>>` where the Wasmer 6 side returns a
   builder and a plain `Arc`, and the two have to converge before one crate body can serve
   both. The `VMHooksLegacyAdapter` activation path described under "What the wasmer 2.2 path
   needs" is the other half.
3. **`wasmercommon` in Go**, wrapping the common header: the error buffer, the status mapping,
   the call result and the hook result, so `wasmer2` and `wasmer6` share them instead of each
   carrying a copy. Worth doing *after* `c-api-common`, since the Go types mirror it.
4. A third cgo shim prefix for the new `wasmer2` package, since `w2_` belongs to the legacy one
   for as long as both exist.

The payoff is the comparison the whole structure exists for: **wasmer 2.2 against wasmer 2.2**,
one through each interface set, under the hook-sequence differential. That isolates the
interface migration from the runtime upgrade — if those two agree, any remaining difference
against `wasmer6` is Wasmer's doing and not the new FFI's.

## Phase 5 — Bring-up and validation

- Unit tests per Phase 1 item, plus the FFI boundary test from Phase 2.
- Targeted Go tests for the G2 memory edge cases: the clamped over-read, negative length, low
  and high store bounds. The three current implementations disagree, so these come before
  anything else is trusted. The Rust side of these is already covered by
  `c-api-new/tests/ffi_boundary.rs`; what remains is asserting the Go wrapper agrees.
- **Differential run**: the scenario suite under both backends, asserting identical
  `VMOutput` (return code, message, logs, gas used). This is the acceptance criterion. If G5
  lands, "both backends" is a flag on one library in one process rather than two builds, and
  after Phase 4b both speak the same Go interfaces, so the harness is the same either way.
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

Remaining path: **Phase 4b → Phase 5**, with the `-Bsymbolic-functions` verification
alongside 4b rather than after it.

Phase 4a was independent of everything else and answered the two questions that gated the
rest. The struct-return ABI works end to end, and the backends coexist. Phase 4b never
depended on G5 for the interface *shape* — that was already determined by the Rust side — only
for how the backend gets chosen, so G5's answer does not change it.

**Phase 1.7 is on the critical path; Phase 1.6 came off it.** Phase 4b needs the hooks
provider to take a per-instance handle, which is 1.7, and G4 has to be settled for that. Phase
1.6 — converging the two `RuntimeRef` traits — was only needed in order to serve wasmer 2.2
through the *new* FFI, and G5's answer removed that need: the differential run uses `wasmer2`
over the legacy library, presenting the new Go interfaces. 1.6 is now cleanup for when the
legacy interface set is retired.

**What took its place:** verifying `-Wl,-Bsymbolic-functions` on Linux. Any binary linking both
`libvmexeccapi` and `libvmexeccapi2` — the Phase 5 harness, and later a node that keeps the old
VM module for historical re-execution — has 32 colliding `wasmer_vm_*` symbols between them.
This is the one item that has to land before Phase 5 runs anywhere but macOS.
