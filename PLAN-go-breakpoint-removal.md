# Replacing runtime breakpoints with early exits in the Go VM

**Repo:** `mx-chain-vm-go` (branch `wasmer6`), with supporting changes in `mx-vm-executor-rs`
**Related:**
- `PLAN-go-hooks-error-migration.md` — the prerequisite, **already landed** (see §1).
- `PLAN-go-ffi.md` — Phase 3 (new Go instance interface) is the hard blocker for this work.

**Status:** analysis only. No code has been written for this change.

---

## 1. Where we are

The hook error migration is complete on branch `wasmer6`:

```
c4989db26 Delete unused (*VMHooksImpl).FailExecution method
e307ad8e2 Make the trailing error result mandatory for all VM hooks
2b267172c Migrate baseOps.go hooks to return error — last of 8 files
... (6 more per-file migration commits)
```

All 278 `@autogenerate(VMHooks)` hooks return a trailing `error`. The eight hook files contain
no remaining `FailExecution`, `SignalUserError` or `SetRuntimeBreakpointValue` calls — a grep
for those names in `vmhost/vmhooks/` returns only the `UseGasBoundedShouldFailExecution()`
feature-flag name and two explanatory comments.

**This is the leverage for everything below.** Every hook failure now funnels through one
function, `(*VMHooksImpl).HandleHookError` in `vmhost/vmhooks/earlyExit.go`. Replacing
breakpoints with early exits is a change to `applyEarlyExit` plus the call-result handler. It
is *not* another pass over 278 hooks.

---

## 2. The central observation

The breakpoint global is a `u64` wasm global, polled by the metering middleware at every
basic-block boundary. It currently serves **two unrelated purposes**:

1. **Halt trigger** (mechanical) — nonzero means "trap at the next checkpoint".
2. **Reason channel** (semantic) — `vmhost/hostCore/breakpoints.go:13` reads it back after the
   trap to decide what happened, and maps it to a sentinel error.

Early exits replace **the reason channel only**. The halt trigger is still required in at
least one place (§4.3), so the global does not disappear.

Separating the two is both possible and already proven — see §3. It is what makes a staged
migration possible: *the Go VM can stop interpreting breakpoints long before it stops writing
one.*

---

## 3. The Rust precedent (copy this design)

In the Rust VM, `InstanceStateSetEarlyExit::set_early_exit` does **both** things, and says so:

`mx-sdk-rs-w1/chain/wasmer-prod/src/wasmer_prod.rs:97-107`

```rust
impl InstanceStateSetEarlyExit for WasmerProdInstanceStateAdapter {
    fn set_early_exit(&self, early_exit: VMHooksEarlyExit) {
        // This ensures that the execution will halt after the current instruction.
        self.0
            .set_breakpoint_value_legacy(early_exit_to_breakpoint_value(&early_exit))
            .expect("set_breakpoint_value_legacy error");

        // This is passed to the error handler at the end of the executor execution.
        self.0
            .set_early_exit(early_exit)
            .expect("set_early_exit error");
    }
}
```

And on the way out, the early exit **wins**:

`mx-vm-executor-rs/vm-executor-wasmer/src/new_traits/wasmer_prod_instance.rs:39-64`

```rust
Err(err) => {
    if let Some(early_exit) = self.inner_instance_ref.take_early_exit() {
        return InstanceCallResult::VMHooksEarlyExit(early_exit);
    }
    // only now read the breakpoint global — middleware-originated stops
    ...
}
```

So the value written by `early_exit_to_breakpoint_value` is **never read back**. It is a
doorbell, not a message. `take_early_exit` also consumes the slot, so there is no sticky state
to clear between calls.

### Code numbering, already settled

`mx-sdk-rs-w1/chain/vm/src/host/vm_hooks/vh_early_exit.rs`

```rust
pub const ASYNC_CALL_EARLY_EXIT_CODE: u64 = 0;

pub fn early_exit_out_of_gas() -> VMHooksEarlyExit { ... ReturnCode::OutOfGas.as_u64() ... }
pub fn early_exit_vm_error(message: &'static str) -> VMHooksEarlyExit { ... ReturnCode::ExecutionFailed ... }
pub fn early_exit_async_call() -> VMHooksEarlyExit { ... ASYNC_CALL_EARLY_EXIT_CODE ... }
```

The code is `ReturnCode::as_u64()`, and `0` is reused as the async-call sentinel because a hook
never exits with `Ok`. Go has `vmcommon.ReturnCode` with `Ok = 0`, so **the same sentinel works
unchanged**, and `executor.EarlyExit.Code` is already a `vmcommon.ReturnCode` — it was built for
this.

The consumer to mirror is
`mx-sdk-rs-w1/chain/vm/src/host/runtime/runtime_instance_call_default.rs:55-85`.

---

## 4. Census: every production write site

| # | Site | Value written | Triggered by | Fate |
|---|---|---|---|---|
| 1 | `vmhost/vmhooks/earlyExit.go:50` | `OutOfGas` | hook | → early exit |
| 2 | `vmhost/contexts/runtime.go:614` (`FailExecution`) | `ExecutionFailed` / `OutOfGas` | hook | → early exit |
| 3 | `vmhost/contexts/runtime.go:629` (`SignalUserError`) | `SignalError` | hook | → early exit |
| 4 | `vmhost/contexts/async.go:583` | `AsyncCall` | hook | → early exit, code-0 sentinel |
| 5 | `vmhost/contexts/runtime.go:311` | `None` (reset) | VM bookkeeping | → delete |
| 6 | `vmhost/hostCore/executionAsync.go:10` | `None` (reset) | VM bookkeeping | → delete |
| 7 | `vmhost/hostCore/host.go:455`, `:534` | `ExecutionFailed` | **watchdog goroutine** | → **must stay** |

Plus the reader: `vmhost/hostCore/breakpoints.go` (`handleBreakpointIfAny` /
`handleBreakpoint`), and the interface declarations at `vmhost/interface.go:140-141`.

Test/mock surface: **19** sites in `mock/contracts/` + `testcommon/`, **14** in `_test.go`
files, and `mock/context/instanceMock.go`.

### 4.1 Rows 1-4 — hook-originated

All four are reachable only from hook bodies, so all four are already covered by the single
`HandleHookError` funnel.

Row 4 was verified specifically: `AsyncContext.RegisterLegacyAsyncCall` has exactly four
callers, all hooks — `vmhost/vmhooks/baseOps.go:1608`, `:1688`, `:1746` and
`vmhost/vmhooks/managedei.go:647`.

### 4.2 Rows 5-6 — resets

Both write `BreakpointNone` to clear sticky state. Under the early-exit model the reason is a
return value rather than instance state, and the Rust side's `take_early_exit()` consumes the
slot, so there is nothing to clear. Row 5 belongs to warm-instance startup and row 6 to
`handleAsyncCallBreakpoint`; both become dead once rows 1-4 are converted.

### 4.3 Row 7 — the watchdog, which cannot become an early exit

`vmhost/hostCore/host.go:528-536`:

```go
	case <-ctx.Done():
		// Terminated due to timeout. The VM sets the `ExecutionFailed` breakpoint
		// in Wasmer. Also, the VM must wait for Wasmer to reach the end of a WASM
		// basic block in order to close the WASM instance cleanly. ...
		host.Runtime().FailExecution(vmhost.ErrExecutionFailedWithTimeout)
		<-done
		err = vmhost.ErrExecutionFailedWithTimeout
```

This runs on a **different goroutine** while the executing goroutine is inside
`instance.Call()`. No hook is running, so there is no return value to carry anything. It is a
genuinely asynchronous interruption and it is the reason the halt trigger survives.

**Gap in the new trait set.** The new `BreakpointValue`
(`mx-vm-executor-rs/vm-executor/src/new_traits/breakpoint_value_new.rs`) has only `None`,
`OutOfGas` and `MemoryLimit`, deliberately — its doc comment says it "only covers the
interruptions raised by the executor's own middlewares." A host-requested abort fits neither
that enum nor `VMHooksEarlyExit`. Options:

- **(recommended)** add `Instance::request_stop()` and `InstanceCallResult::Interrupted`. This
  names the thing honestly and is the only variant that is legitimately callable across
  threads, which should be documented on the method.
- add a third `BreakpointValue` variant for host-requested stops — muddies the
  "middlewares only" invariant.

Either way, `SetRuntimeBreakpointValue` should be **removed** from `vmhost.RuntimeContext`
(`vmhost/interface.go:140`) and replaced by a narrow `InterruptExecution()` with exactly one
caller. Leaving a general-purpose breakpoint setter on the interface is what allowed
hook-level failure signalling to grow there in the first place.

### 4.4 Stragglers that are not hooks

`vmhost/contexts/blockchain.go:353` — `PopSetActiveState` calls `vmhooks.FailExecution` when
`RevertToSnapshot` fails, during state unwinding, outside any hook. Either plumb an error out
of `PopSetActiveState` or give it the watchdog treatment. One site, but it will not fall out of
the hook model.

`vmhost/vmhooks/vmHooksImpl.go:100-110` — the package-level `vmhooks.FailExecution(host, err)`
stays exported for mocks, `testcommon`, and `blockchain.go`. 17 external callers.

---

## 5. Breakpoint values after the migration

The point of the exercise: each value ends up with exactly one producer.

| Value | Producer after migration |
|---|---|
| `BreakpointNone` (0) | — |
| `BreakpointExecutionFailed` (1) | the watchdog only (§4.3), or gone if `Interrupted` is added |
| `BreakpointAsyncCall` (2) | **nobody — delete** |
| `BreakpointSignalError` (3) | **nobody — delete** |
| `BreakpointOutOfGas` (4) | metering middleware only |
| `BreakpointMemoryLimit` (5) | memory middleware only |

Rows 2 and 3 going to zero producers is the **completeness check** for this work: grep for
`BreakpointAsyncCall` and `BreakpointSignalError` and expect only the definitions in
`vmhost/common.go` and their `String()` cases.

Note that `BreakpointOutOfGas` has two producers today (middleware *and* VM-injected, via both
`earlyExit.go:50` and the `errors.Is(err, ErrNotEnoughGas)` upgrade inside
`runtime.FailExecution`). After the migration the VM-side one becomes
`EarlyExit{Code: OutOfGas}`. Both still map to `ErrNotEnoughGas`, so behaviour converges — but
they become distinguishable, which is an improvement for diagnostics.

---

## 6. What it takes

### 6.1 Hard blocker: a Go-side `InstanceCallResult`

`vmhost/contexts/runtime.go:813`:

```go
func (context *runtimeContext) CallSCFunction(functionName string) error {
	return context.iTracker.Instance().CallFunction(functionName)
}
```

`executor.Instance` (`executor/executorInstance.go:12`) offers only
`CallFunction(functionName string) error`. **There is nowhere for a reason to travel.** Until
the new instance interface lands (`PLAN-go-ffi.md` Phase 3), the breakpoint global is the only
channel and none of this work can start.

Sketch of the target, mirroring `InstanceCallResult` in Rust:

```go
// package executor
type InstanceCallResultKind uint8

const (
	CallResultOk InstanceCallResultKind = iota
	CallResultFunctionNotFound
	CallResultRuntimeError
	CallResultEarlyExit
	CallResultBreakpoint
	// CallResultInterrupted — if the request_stop design in §4.3 is adopted
)

type InstanceCallResult struct {
	Kind       InstanceCallResultKind
	Err        error           // CallResultRuntimeError
	ExitCode   uint64          // CallResultEarlyExit — vmcommon.ReturnCode, or 0 = async call
	ExitMsg    string          // CallResultEarlyExit
	Breakpoint BreakpointValue // CallResultBreakpoint — OutOfGas | MemoryLimit only
}
```

Then `handleBreakpointIfAny(err)` becomes a **pure function** `resolveCallResult(result) error`
with no global reads. The three call sites are already identical in shape and convert together:
`vmhost/hostCore/execution.go:928`, `:1173`, `:1290`.

The async-call branch is the one that returns `nil`, preserving
`handleAsyncCallBreakpoint`'s "stopped intentionally, continue with the async call" semantics.

### 6.2 `ConsumeAllGas` does not cross the FFI

`VMHooksEarlyExit` carries only `code` and `message`. `executor.EarlyExit.ConsumeAllGas` is a
Go-side concept, so the burn must stay in `HandleHookError`, executed before the hook returns.
`applyEarlyExit` therefore keeps a gas responsibility even after it loses its breakpoint
responsibility. Do not try to encode the flag in the exit code.

### 6.3 Mocks are roughly a third of the work

`mock/context/instanceMock.go:63-76` currently *polls* the breakpoint global and reconstructs
an error message from the VMOutput:

```go
func (instance *InstanceMock) CallFunction(funcName string) error {
	err := instance.DefaultErrors[funcName]
	method := instance.Methods[funcName]
	newInstance := method()
	if vmhost.BreakpointValue(instance.GetBreakpointValue()) != vmhost.BreakpointNone {
		...
	}
	return err
}
```

It is emulating the middleware. It needs its own early-exit slot so that mock contracts can
return exits the way real hooks now do. Mechanical, but it touches many files — and these tests
are the only proof the migration is behaviour-preserving.

---

## 7. Recommended staging

Split along the semantic/mechanical seam.

### Step A — stop *interpreting* breakpoints (no wasmer 7 dependency)

1. Land the Go `InstanceCallResult` and the new instance interface.
2. Convert the three `handleBreakpointIfAny` sites to `resolveCallResult`.
3. Have `applyEarlyExit` record the exit on the instance **in addition to** setting the
   breakpoint — exactly mirroring `wasmer_prod.rs:97-107`.
4. Delete the reset sites (rows 5-6).

Breakpoint writes still happen; nothing reads their *value* except the middleware fallback.
`BreakpointAsyncCall` and `BreakpointSignalError` become dead (§5). Behaviour-preserving and
fully testable on wasmer 2 today. This is the larger change.

### Step B — stop *writing* them (needs the new FFI)

1. On the new FFI the exit travels in the hook's return value and the Rust side halts, so
   `applyEarlyExit` loses its breakpoint branch entirely.
2. Delete `SetRuntimeBreakpointValue` from `vmhost.RuntimeContext`; add the narrow
   `InterruptExecution()` for the watchdog (§4.3).
3. Resolve `blockchain.go:353` (§4.4).

Small, and it depends on the new FFI crate.

---

## 8. Risks

**Halt timing changes in Step B.** Today a hook that fails keeps executing wasm until the
middleware next polls the global; with a real trap the halt is immediate. This is observable in
points-used, and therefore in nested-execution gas accounting. It is a *correctness
improvement*, but it is a behaviour change and needs differential testing against the wasmer 2
path, not just unit tests.

**The sentinel collision is load-bearing.** `ASYNC_CALL_EARLY_EXIT_CODE = 0` works only because
no hook ever exits with `Ok`. If a future hook wants to exit with `Ok`, the numbering breaks
silently on both sides of the FFI. Worth an assertion at the point where an `EarlyExit` is
constructed.

**`BreakpointExecutionFailed` must not quietly acquire new producers.** After Step B its only
producer is the watchdog. A code review rule or a lint on `InterruptExecution` callers is
cheaper than rediscovering this in a year.

---

## 9. Verification greps

```bash
cd mx-chain-vm-go

# must reach zero producers (definitions in vmhost/common.go excepted)
grep -rn "BreakpointAsyncCall\|BreakpointSignalError" --include="*.go" .

# the breakpoint setter: 7 production sites before, 1 narrow interrupt after
grep -rn "SetRuntimeBreakpointValue(" --include="*.go" vmhost/ executor/ wasmer2/ | grep -v _test

# the hook funnel must stay the only place failure effects are applied
grep -rn "FailExecution\|SignalUserError\|SetRuntimeBreakpointValue" \
  --include="*.go" vmhost/vmhooks/ | grep -v _test | grep -v earlyExit.go
# expect: only UseGasBoundedShouldFailExecution() flag hits, 2 comments,
#         and the package-level vmhooks.FailExecution in vmHooksImpl.go

# hook count, for sanity
grep -rn "@autogenerate(VMHooks)" --include="*.go" vmhost/vmhooks/ | wc -l   # 278
```
