# Instance caching in the Rust VM, and what it means for the executor traits

Goal as set: bring the Rust VM up to the Go VM's level — contract code compiled once and
reused — and implement the executor side that this needs. Two halves: the experimental
(Wasmer 6) executor, and the Rust VM itself.

The first thing done was to measure what reuse is worth, because that determines the shape of
both halves. It turned out to change the plan substantially, so it comes first.

## The measurement

A real contract, `test/delegation/v0_4_genesis` (55 KB of wasm), 50 iterations, release build,
Wasmer 6.1 Singlepass on an arm64 Mac. Numbers vary ±20% between runs; the *ratios* do not.

| Operation | Cost | Share of a cold instance |
| --- | --- | --- |
| Compile + instantiate — the path taken today, with all four middlewares | ~6 ms | 100% |
| Compile only, **without** middlewares (a lower bound on the compile share) | ~1.7 ms | — |
| **Instantiate from a `Module` already in memory** | **~30–50 µs** | **under 1%** |
| Deserialize a serialized artifact (427 KB) | ~45 µs | under 1% |
| Hash the wasm bytes, for a cache key | ~14 µs | 0.2% |

The middlewares roughly triple compile time, which is why the full path costs ~6 ms where a
bare Singlepass compile costs ~1.7 ms.

**Compilation is essentially the entire cost; instantiation is noise.** Everything below
follows from that.

### Consequence 1: cache modules, not instances

The Go VM keeps a *warm instance* cache and reuses live instances via `Instance.Reset()`. On
Wasmer 6 that model buys ~30–50 µs on a ~6 ms operation — under 1% — over simply instantiating
a cached module afresh. It is not worth its complexity, and the complexity is considerable:

- **Wasmer 6 has no instance reset, and adding one means forking it.** `Memory::reset` exists
  but, per its own doc, "resets the memory back to zero length" — it does not re-apply data
  segments and does not touch globals. The Wasmer 2.2 `reset()` the Go VM relies on lives in
  the **MultiversX fork** (`handle.reset(data_initializers)`, reaching into `wasmer-vm`).
  Wasmer 6 is consumed from **crates.io, unmodified** — which is a property worth keeping.
- Re-instantiating is **behaviourally identical** to resetting: both yield fresh globals and
  memory initialised from the module's data segments. Nothing observable to a contract
  distinguishes them.

So: `reset()` stays unimplemented, deliberately, with the measurement recorded as the reason.

### Consequence 2: instance caching stays possible, so the traits must allow it

Module caching is what pays, but instance caching must not be *designed out*, because the
C ABI is expensive to change later. The constraint it imposes comes from the fact above: an
instance keeps its hooks object for life, so

> an instance is safely reusable **iff** its hooks resolve the execution context when they are
> called, not when the instance is created.

That is one axis. Where the hooks are *attached* — per executor or per instance — is a second,
independent one. Only the first decides cache safety.

**Decided: hooks per executor, context resolved at call time.** The executor is given a
runtime reference once, at construction, and asks it for hooks on every host call. Instances
then carry no host identity at all, which makes caching them safe by construction rather than
by discipline. It is also what the Go VM already does (`vmHooksProvider { host }`), so the two
VMs converge.

The alternative that was considered and rejected — passing a per-instance hooks provider to
`new_instance` — gives stateless instantiation without a context stack, but binds one
execution into the instance and so forecloses instance reuse. Two further variants (per
instance + resolve-at-call; capture + an explicit `Instance::reuse(hooks)` rebind) each pay
this option's cost without its guarantee, or add an entry point across the FFI for a sub-1%
win.

Three things follow, and two of them are savings:

- The Rust VM's `set_executor_context(Some)/(None)` slot is replaced by a **context stack**
  held for the whole execution, not an arming window around instantiation. That removes the
  current unenforced invariant ("nobody touches the slot between `set` and `new_instance`").
- **No `Any` downcast anywhere.** The escape hatch the per-instance design needed for
  `ContractDebugExecutor` is unnecessary: that executor already holds a `RuntimeWeakRef`, so
  it reads the current context from the stack directly.
- `Executor::new_instance` takes only the wasm and the options.

## The runtime interface

The one thing the executor needs from its host, and therefore the only method on the trait:

```rust
/// The host behind an executor. One per executor, for the executor's whole life.
pub trait ExecutorRuntime: Send + Sync {
    /// Builds the VM hooks servicing one host call, around the instance state for that call,
    /// and lends them to `f` — which is called exactly once.
    ///
    /// Built per call, not per instance: under Wasmer 6 the instance state is only reachable
    /// for the duration of a call, so hooks that hold it cannot outlive it. Lending rather
    /// than returning avoids a `Box<dyn VMHooks>` allocation on every hook invocation.
    ///
    /// Which execution the hooks serve is resolved *here*, by the host, from its own call
    /// stack. That is what makes a cached instance safe to reuse: the instance contributes
    /// the state, never the identity.
    fn with_vm_hooks(
        &self,
        instance_state: &mut dyn InstanceState,
        f: &mut dyn FnMut(&mut dyn VMHooks),
    );
}
```

Object-safe, so an executor stores `Arc<dyn ExecutorRuntime>`. No opcode config and no
compilation options on it — those are arguments to `new_instance`.

### Over the FFI

The runtime reference becomes a per-**executor** handle, where today it is per-instance:

```c
vm_exec_status_t vm_exec_w6_new_executor(
    vm_exec_w6_executor_t **executor_out,
    uintptr_t runtime_ref,                              /* NEW: the host's runtime handle */
    const vm_exec_vm_hook_c_func_pointers *hook_table,
    char *err_buf, int err_buf_len);

vm_exec_status_t vm_exec_w6_new_instance(
    vm_exec_w6_executor_t *executor,
    vm_exec_w6_instance_t **instance_out,
    const uint8_t *wasm_bytes, uint32_t wasm_bytes_len,
    const vm_exec_compilation_options_t *options,       /* now carries the opcode config */
    char *err_buf, int err_buf_len);                    /* go_vm_hooks is gone */
```

`uintptr_t` for the same reason as before: from Go it is a `runtime/cgo.Handle`, and cgo
forbids storing a Go pointer in C memory. `vm_exec_hook_context_t.go_vm_hooks` is renamed to
`runtime_ref` and now carries the executor's handle rather than the instance's.

**The hook context pool becomes thread-local.** It is per-instance today. It cannot become
per-executor behind a `RefCell`, because `vmHost.RunSmartContractCall` takes only an `RLock`
— concurrent executions against one host are permitted, so a shared pool would race. A call
stack is per-thread anyway, and cgo keeps a goroutine on its OS thread for the duration of a
call, which the C API already states as an invariant. Thread-local is also strictly safer than
today: the pool outlives every instance, closing the window in which a context pointer leaked
past instance destruction would dangle rather than merely fail its token check.

## Recursion: the same contract twice on one stack

A contract that calls itself — directly, or round a cycle — needs two live instances of the
same code at once. An instance cache must not hand the inner frame the instance the outer
frame is running, because preparing it for reuse resets memory and globals, which would
destroy the outer frame mid-execution.

**What the Go VM does today**, three guards, all keyed on the stack:

1. *Do not take a warm instance* when the code hash or the SC address is already on the stack
   — `useWarmInstanceIfExists` → `isContractOrCodeHashOnTheStack` (`runtime.go:297`). The
   recursive call falls through to the compiled-code path, and then to bytecode.
2. *Do not save as warm* when the code hash is on the stack — `saveWarmInstance` →
   `IsCodeHashOnTheStack` (`runtime.go:366`). The duplicate never enters the cache.
3. *Destroy the duplicate on the way out* — `PopSetActiveState` cleans the popped instance
   when it is not the one on top of the stack and its code hash is on the stack
   (`instanceTracker.go:97`).

Bounded by `maxInstanceStackSize`, checked at the top of `StartWasmerInstance`.

So the answer to "two copies from the same cache bytes" is **yes, deliberately**: the inner
frame deserializes the artifact again and gets its own memory and globals. Correct, but it
pays a deserialize (~45 µs) per recursion level.

**Under a module cache the problem does not arise.** What is cached is the compiled module,
which is immutable: two frames each call `Instance::new` on the same `Module` and get separate
memories and globals, sharing nothing mutable. No on-stack check is needed for correctness —
guard 1 becomes vacuous because there is no warm instance cache, and guards 2 and 3 have
nothing to protect. It is also *faster* than the current recursion path: one compile, then
~30–50 µs per level instead of a deserialize per level.

The one constraint to respect is the one already stated in 1.1: an `Engine` may be reused to
**instantiate** the module it compiled, but never to **compile** a second module, because the
metering middleware keeps `global_indexes` populated at compile time. Instantiation does not
run middlewares, so recursion is unaffected.

**On the Go side today this is all vacuous anyway**: `Wasmer6Instance.Reset()` returns false,
and both `Cache()` and `NewInstanceFromCompiledCodeWithOptions` return `ErrCacheUnsupported`,
so every execution compiles from bytecode and no instance is ever shared. Recursion is safe
and slow. That is what the module cache fixes.

**Hooks under recursion** need no extra mechanism either. With hooks attached per executor and
the execution resolved at call time (G6), the inner frame's hooks resolve to the inner
context and the outer frame's to the outer, through the host's context stack — which is what
that stack is for. On the C side the hook context pool is already a stack: the inner
activation is innermost, the outer context is refused while it runs and valid again
afterwards, which `probe_nesting` in `c-api-wasmer6/tests/ffi_boundary.rs` already pins.

## Wasmer 7.4, checked alongside — and one reason to move

Everything above was re-verified against Wasmer 7.4, by source diff and by running a real
serialize → deserialize → instantiate round trip (49 exports intact).

- **`serialize`/`deserialize` are unchanged**, in signature and in behaviour. Deserialize
  ~43 µs, instantiate ~31 µs, artifact 436 KB — the same profile as 6.1. The caching design
  does not depend on which of the two we are on.
- **The `deserialize` quirk persists**: it still delegates to `deserialize_unchecked`, so the
  "checked" name still validates nothing. Artifacts must come from a trusted store.
- **Still no instance reset.** `Memory::reset` carries the same "resets the memory back to
  zero length" semantics, with no data-segment or global restore.
- Artifact ABI version 11 → 24, as noted in 1.3.

**The reason to move is unrelated to caching.**
`vm-executor-experimental/src/middlewares/we_metering.rs` has `feed_local_count` commented out
under a TODO listing three options ("find alternative / PR to Wasmer / fork Wasmer"), because
Wasmer 6 has no hook for it. The Wasmer 2.2 backend has that function **live**. So the two
backends currently disagree on gas: **Wasmer 6 does not charge the unmetered-locals cost that
Wasmer 2.2 charges.**

Wasmer 7.4 adds `FunctionMiddleware::locals_info(&mut self, locals: &[ValType])` — option (b)
of that TODO, landed upstream. It is fed from `read_local_decl`, which expands each
declaration group, so `locals.len()` is exactly the old `count`, delivered once before the
first operator. One wrinkle: it returns `()`, so `check_local_count_exceeded`'s error has to be
stashed and reported from the first `feed`.

The cost of the upgrade is that `FunctionMiddleware` gained a lifetime parameter
(`FunctionMiddleware<'a>`, `generate_function_middleware<'a>() -> Box<dyn FunctionMiddleware<'a> + 'a>`,
`feed` losing its own `'a`), which is mechanical but touches all five middlewares. The rest of
the API we use — `FunctionEnvMut`, `MemoryView`, `Store`, the `Global` helpers, the Singlepass
config, trap and error types — has not been audited; that needs a compile, which is a contained
experiment.

Worth pointing the hook-sequence differential at contracts with many locals either way, since
that is where the divergence would show.

## Side 1 — the experimental executor

### 1.1 Separate compilation from instantiation

`ExperimentalInstance::try_new_instance` currently does `Store::new(compiler)` → `Module::new`
→ instantiate in one call. Split it:

```rust
/// A contract compiled and ready to instantiate. Holds no host state.
pub trait CompiledModule: Send + Sync {
    fn serialize(&self) -> Result<Vec<u8>, ExecutorError>;
}

pub trait Executor {
    fn compile(&self, wasm: &[u8], options: &CompilationOptions)
        -> Result<Arc<dyn CompiledModule>, ExecutorError>;

    fn instantiate(&self, module: &dyn CompiledModule)
        -> Result<Box<dyn Instance>, ExecutorError>;

    /// Convenience, and the cache's entry point: compile-or-reuse, then instantiate.
    fn new_instance(&self, wasm: &[u8], options: &CompilationOptions)
        -> Result<Box<dyn Instance>, ExecutorError>;
}
```

The Wasmer 6 implementation holds `{ engine: Engine, module: Module }`. The `Engine` has to
travel with the `Module`, because instantiation needs a `Store` built on the same engine
(`Store::new(engine.clone())` — the clone is an `Arc` bump).

**One engine per compiled module, not one shared engine.** The metering middleware keeps
`global_indexes: Mutex<Option<MeteringGlobalIndexes>>`, populated by `transform_module_info`
during compilation. Compiling a second contract through the same engine would overwrite the
first one's indexes. Instantiation does not run middlewares, so *reusing* an engine to
instantiate the module it compiled is safe; reusing it to compile is not.

### 1.2 The module cache

In the executor, not in the VM — it is pure memoization with no host semantics, so every host
gets it without writing any cache code.

- **Key: `(hash(wasm_bytes), hash(options))`.** Including the options makes the cache
  self-invalidating: a gas-schedule change simply misses, with no "remember to clear the
  cache" discipline anywhere. That is strictly better than the Go VM's model, where
  `GasScheduleChange` clears the warm instance cache but the persisted compiled-code cache is
  cleared only at flagged activation epochs.
- **Bound it with an LRU.** Artifacts are ~8× the wasm size (427 KB for 55 KB of wasm), so an
  entry count alone is a poor bound; prefer a size-aware cap.
- Hashing costs ~14 µs on 55 KB with a non-cryptographic hasher — a third of an
  instantiation. Acceptable. Use a fast hash, not a cryptographic one: this key never leaves
  the process and is not consensus-relevant.

*Decision deferred:* the Go VM already has a code hash and would be paying that 14 µs twice.
If it shows up, add an optional caller-supplied key. Not worth the extra parameter up front.

### 1.3 The serialized-artifact path

`cache()` already works (`module.serialize()`). Finish the other half: `new_instance_from_cache`
should do `Module::deserialize(&engine, bytes)` against an engine built from the same options.

**Security note to carry into the doc comment:** in Wasmer 6.1, `Module::deserialize` delegates
to `deserialize_unchecked` — the "checked" variant performs no validation. Artifacts must come
from a trusted store. That is true of our own cache, but it must be stated, because the name
suggests otherwise.

**Artifacts are not portable across Wasmer versions.** The serialized header carries an ABI
version which `deserialize` rejects on mismatch, and it moved from **11** (Wasmer 6.1) to
**24** (Wasmer 7.4). Any persistent cache therefore needs the Wasmer version in its key, or an
explicit purge on upgrade. Irrelevant to the in-memory module cache, which never serializes.

This tier matters less for the Rust VM (in-process, so the module cache covers it) than for
persistence across process restarts. Lower priority than 1.1 and 1.2.

### 1.4 What is explicitly not built

`reset()` stays `Err(ResetUnsupported)`, with the measurement and the fork argument recorded in
the doc comment so the question is not reopened without new evidence.

## Side 2 — the Rust VM

Smaller than expected, because the cache lives in the executor. Mostly replacement and removal.

### 2.1 A context stack in place of the arming slot

`Runtime::context_cell: RwLock<Option<TxContextRef>>` is set before `new_instance` and cleared
straight after, so the hooks capture the transaction while the window is open. Replace it with
a stack pushed for the whole of `RuntimeRef::execute` and popped at the end:

- `push_context(tx_context_ref)` / `pop_context()`, with `current_context()` reading the top.
- Nesting works the way it already does in the Go VM: a hook that re-enters the VM pushes its
  own context and pops it on the way out, and the outer context is on top again afterwards.
- The unenforced "nobody touches the slot between `set` and `new_instance`" invariant goes
  away, because nothing depends on a window any more.

`impl ExecutorRuntime for RuntimeWeakRef` then has one method, which resolves the current
context from that stack and builds the dispatcher around the state it was handed. One
implementation serves both backends — the Wasmer 2.2 and Wasmer 6 executors are configured
identically.

### 2.2 `ContractDebugExecutor` needs nothing special

It runs Rust closures rather than wasm and needs the `TxContextRef` itself. It already holds a
`RuntimeWeakRef`, so it reads the current context from the stack. No downcast, no `Any`
supertrait, no escape hatch — which was the weak point of the rejected per-instance design.

### 2.3 Wiring and measurement

- `CompositeExecutor` forwards only the options; there is no provider to thread through.
- No VM-side cache code, no instance tracker, no instance stack.
- **Measure on a real scenario suite** before and after. The expected win is the compile share
  of every repeated execution — ~6 ms per call collapsing to ~50 µs for every call after the
  first with the same code. If the observed win is far from that, the cache is not being hit
  and the key is wrong.

## Sequencing

| Stage | Work | Depends on |
| --- | --- | --- |
| 0 | ~~`ExecutorRuntime` on the executor; `OpcodeConfig` folded into `CompilationOptions`; `new_instance(wasm, options)`~~ — **done**, see below | — |
| 1 | Executor: split compile from instantiate; `CompiledModule` | 0 |
| 2 | Executor: module cache, keyed on wasm + options | 1 |
| 3 | Rust VM: context stack, `impl ExecutorRuntime for RuntimeWeakRef`, delete the slot | 0 |
| 4 | Measure on a scenario suite | 2, 3 |
| 5 | Serialized-artifact path (`new_instance_from_cache`), keyed including the Wasmer version | 1 |

Stages 1–2 and 3 are independent once 0 lands.

### Stage 0 as built

`ExecutorRuntime` landed as drafted above, with one method. What the implementation added to
the sketch:

- **`impl<T: InstanceState + ?Sized> InstanceState for &mut T`.** Without it a host that is
  generic over `S: InstanceState` cannot accept the `&mut dyn InstanceState` the runtime lends
  it, and every host needs its own forwarding wrapper — which puts the backend's concrete
  state type back into the host, the one thing the erased state exists to prevent. The Rust
  VM's `WasmerProdInstanceStateAdapter` is exactly that wrapper, written out by hand.
  `ExperimentalInstanceState` now implements `InstanceState` on the struct rather than on
  `&mut` itself, so the blanket impl covers the reference.

- **The C API stopped naming a backend entirely.** `capi_backend.rs` is deleted and
  `CapiVMHooksBuilder` implements `ExecutorRuntime` directly; see `PLAN-go-ffi.md` Phase 4c.

- **Both backends converged, not just Wasmer 6.** `WasmerProdRuntimeRef` is deleted;
  `WasmerProdExecutor` takes the same `Arc<dyn ExecutorRuntime>`. The obstacle was that legacy
  `VMHooksLegacy` is an *object* built once per instance, with nowhere to report an early exit,
  while the runtime *lends* hooks per call. Resolved by regenerating `VMHooksLegacyAdapter`
  over `R: ExecutorRuntime` + `S: InstanceState` — header template only, all 277 bodies
  byte-identical — so 2.2 now resolves the execution when the hook runs, exactly as Wasmer 6
  does. The stash-and-halt half is a defaulted `InstanceState::handle_early_exit_legacy`,
  which fails for every backend that does not implement it; a probe removing the 2.2 override
  fails exactly the OutOfGas and UserError tests and leaves the no-early-exit one passing. This was not optional tidying:
  `c-api-common`'s shared runtime *is* an `ExecutorRuntime`, so a 2.2 capi built on it could
  not have existed otherwise. In the Rust VM it deleted `RuntimeRefAdapter`,
  `WasmerProdInstanceStateAdapter` and the whole `InstanceStateSetEarlyExit` /
  `VMHooksSetEarlyExit` pair, and `impl ExecutorRuntime for RuntimeWeakRef` moved out of the
  `wasmer-experimental` feature gate to serve both.

- **The arming slot had to become a stack, early.** This was Stage 3's item, but Stage 0
  forced its core: resolving the execution per hook call means the slot must stay armed for
  the whole call, where it used to be cleared as soon as the instance was built. Widening the
  window naively breaks nesting — a nested `execute` would clear its caller's slot on the way
  out — so `RuntimeRef::execute` now saves the previous value and restores it on `Drop`. The
  saved values form the stack, held on the Rust call stack. What remains for Stage 3 is
  deleting the slot in favour of reading the stack directly.

- **The C ABI is unchanged, deliberately.** The opcode config has left the executor in Rust,
  but `vm_exec_executor_config_t` stays and the executor merges it into every instance's
  options. Moving it across the ABI is a coordinated change with the Go VM, and it rides with
  the runtime-handle move rather than going separately.

**Verification.** Executor workspace: 19 test binaries green, clippy clean, the C header diff
comment-only. The 2.2 backend passes its gas tests unchanged, including the OutOfGas and
UserError early-exit cases that exercise the new adapter path. Rust VM: the gas tests pass on both backends with their exact gas assertions
(`gas_used == 2045`), and 250 `basic-features` tests pass against compiled wasm through the
experimental executor. Go VM: the rebuilt dylib drops in with no Go changes and the
`VMEXECUTOR=wasmer6` failure set is identical before and after — 33 failures, same set,
compared by stashing the dylib.

One check worth recording, because it nearly produced a false pass: the composability suite
(332 tests, contract-to-contract and recursive callers) runs on the **Debugger** by default,
not the experimental executor, so it stays green even with the context guard deliberately
broken. The suites that actually exercise this path are the ones calling
`ExecutorConfig::full_suite()` or `::Experimental`, and then only with `compiled-sc-tests`
active. Dropping the guard early makes `scenario-tester` fail with *no executor context
configured*, which is what establishes the guard is load-bearing.

**One cross-repo constraint on the C API.** Moving the runtime handle from per-instance to
per-executor requires **`PLAN-go-ffi.md` Phase 4b's `MemLoad`/`MemStore` removal** to land
first. Go's per-instance
`cgo.Handle` resolves to `instanceHooks`, which holds `activeState`; that slot's own comment
says the nesting argument depends on it being per instance ("a nested call runs on a different
instance"). It exists only because the VM still reaches wasm memory through
`executor.Instance` rather than through the state it was handed, which is exactly what that
item removes. Until then the C API keeps its per-instance handle and the capi keeps bypassing the
`Executor` trait. The Rust-side stages above are not blocked by this; only the FFI change is.

## What this means for the Go VM

Not in scope here, but the measurement applies to Go's Wasmer 6 backend too, and it bears on
`PLAN-go-ffi.md` Phase 6 ("`reset()` on the Wasmer 6 instance — prerequisite for Go's
warm-instance tracker").

It is not a prerequisite, because it is not worth building. The Go VM's existing
*compiled-code* path (`NewInstanceFromCompiledCodeWithOptions`) is already the
Wasmer-6-compatible form of reuse: for this backend the warm instance cache collapses into the
module cache. The cheapest route is for `Wasmer6Executor` to keep compiled modules in memory
and let `Reset()` keep returning false, which the instance tracker already handles by evicting
and recreating.

Worth confirming separately, because it is the one place where the Go VM's reuse model and
Wasmer 6's capabilities genuinely disagree.
