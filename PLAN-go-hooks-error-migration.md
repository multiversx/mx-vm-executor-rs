# Migrating the Go VM hooks to return `error`

**Repo:** `mx-chain-vm-go` (branch `wasmer6`)
**Related:** `PLAN-go-ffi.md` in `mx-vm-executor-rs` — this document is the Go-side prerequisite for its Phase 3.
**Status:** done, and left as the work order it was. One thing has since moved: the `EarlyExit`
type now lives in `executor`, not `vmhost`, so that an executor reporting early exits through
its own call result can read the code off it. Everything below still describes the shape
accurately apart from that package. See Phase 3 of `PLAN-go-ffi.md`.

---

## 1. Goal

Today every VM hook that fails performs the failure side effects itself — it calls
`FailExecution` (888 call sites) or sets a runtime breakpoint (6 sites) — and then returns a
value. After this migration every hook **returns** its failure as an `error` and performs no
failure side effects. The generated hook wrapper applies the effects.

The motivation is the new executor interface (`vm-executor/src/new_traits`), where a hook
reports early exit through its return value instead of through a breakpoint global. That
transport change comes later; this migration only moves the *decision* out of the hook body,
so that the later change has a single place to hook into.

### Invariant for this migration

**This migration must be observably a no-op.** Same return codes, same return messages, same
gas accounting, same error traces, same breakpoints, for every existing test. Any behaviour
change is a bug in the migration, not an improvement. Several tempting cleanups are called
out in §9 as *deliberately deferred*.

### Scope

**In scope**

- All 277 `@autogenerate(VMHooks)` methods in `vmhost/vmhooks/` (see §2 for the census).
- The unexported and `…WithHost` / `…WithTypedArgs` helpers those hooks call, where they
  currently perform failure side effects.
- New hand-written support code: `EarlyExit`, `resolveEarlyExit`, `applyEarlyExit`,
  `HandleHookError`.
- The code generator in `vmhost/vmhooks/generate/` and its **Go** outputs.
- Go call sites of hooks and helpers in tests and `mock/contracts/`.

**Out of scope — do not touch**

- The C ABI. The `// extern …` declarations in `wasmer2/wasmer2ImportsCgo.go` keep their
  current signatures (`void` / `int32_t` / `long long`). The error is consumed on the Go side.
- Everything in `mx-vm-executor-rs` and `mx-sdk-rs`. All Rust generator writers
  (`eiGenWriteRust*.go`) and all opcode-cost writers must keep producing **byte-identical**
  output. This is a verification step, see §8.
- `libvmexeccapi.h`, `libvmexeccapi.*` binaries.
- `vmhost/contexts/runtime.go`: `FailExecution` and `SignalUserError` stay exactly as they are.

---

## 2. Census — what you are changing

`@autogenerate(VMHooks)` appears 278 times but there are **277 hooks**: `UpgradeContract`
(`baseOps.go:1516-1517`) has the annotation twice. Remove the duplicate line as part of the
migration.

| source file | void | int32 | int64 | total | `FailExecution` sites |
|---|---:|---:|---:|---:|---:|
| `baseOps.go` | 21 | 46 | 20 | 87 | 234 |
| `managedei.go` | 26 | 15 | 0 | 41 | 119 |
| `bigIntOps.go` | 30 | 11 | 1 | 42 | 126 |
| `manBufOps.go` | 3 | 22 | 2 | 27 | 88 |
| `cryptoei.go` | 2 | 39 | 0 | 41 | 162 |
| `bigFloatOps.go` | 16 | 6 | 0 | 22 | 73 |
| `smallIntOps.go` | 3 | 3 | 6 | 12 | 22 |
| `manMapOps.go` | 0 | 5 | 0 | 5 | 9 |
| **total** | **101** | **147** | **29** | **277** | **833** |

Plus 4 sites in `vmHooksImpl.go` (the definition and the method wrapper), 1 direct
`runtime.FailExecution(` inside `vmhooks`, and 6 `SetRuntimeBreakpointValue(BreakpointOutOfGas)`
sites (`baseOps.go:1694,1715,1783,1801,1859`, `managedei.go:679`).

Call-site forms in use: `context.FailExecution(` (571), `FailExecution(` package-level (265),
`runtime.FailExecution(` (1).

Only **two** types implement `executor.VMHooks`:
`vmhost/vmhooks.VMHooksImpl` and the generated `executor/wrapper.WrapperVMHooks`. There are
~115 hook/helper call sites in `*_test.go` and `mock/contracts/`.

---

## 3. Target shape

### 3.1 Hook signatures

| today | after |
|---|---|
| `func (context *VMHooksImpl) X(args…)` | `func (context *VMHooksImpl) X(args…) error` |
| `func (context *VMHooksImpl) X(args…) int32` | `func (context *VMHooksImpl) X(args…) (int32, error)` |
| `func (context *VMHooksImpl) X(args…) int64` | `func (context *VMHooksImpl) X(args…) (int64, error)` |

The declared error type is the plain `error` interface, **not** `*vmhost.EarlyExit`. This is
deliberate:

- The ~833 failure sites keep returning the existing package-level sentinels
  (`vmhost.ErrBadBounds`, `vmhost.ErrNoManagedBufferUnderThisHandle`, …) with no wrapping and
  no allocation.
- `errors.Is` / `errors.As` / `fmt.Errorf("%w")` keep working end to end.
- The many helpers that already return `error` compose unchanged.
- It avoids the typed-nil trap (`var e *EarlyExit = nil; var err error = e; err != nil`).

`*vmhost.EarlyExit` is just *one* implementation of `error`, used only by the handful of hooks
that need non-default failure behaviour, and recovered with `errors.As`.

### 3.2 New hand-written code

Three new pieces. Create them first, before touching anything else.

**(a) `vmhost/earlyExit.go`** — package `vmhost`, new file.

```go
package vmhost

import vmcommon "github.com/multiversx/mx-chain-vm-common-go"

// EarlyExit is an error that describes how execution must be terminated from
// inside a VM hook. Hooks only need it for the cases that deviate from the
// default failure behaviour; for everything else a plain error is enough and
// resolveEarlyExit supplies the default.
type EarlyExit struct {
	// Code selects which termination routine is applied. Only
	// vmcommon.ExecutionFailed, vmcommon.UserError and vmcommon.OutOfGas are
	// meaningful today.
	Code vmcommon.ReturnCode

	// Message is the VMOutput return message. For a user error this is the
	// contract's own text, which is not the same as Cause.Error().
	Message string

	// ConsumeAllGas burns all remaining gas before the termination routine runs.
	ConsumeAllGas bool

	// Cause is the underlying error, used for the error trace and errors.Is.
	Cause error
}

// Error implements the error interface.
func (e *EarlyExit) Error() string { return e.Message }

// Unwrap keeps errors.Is/errors.As working through an EarlyExit.
func (e *EarlyExit) Unwrap() error { return e.Cause }

// ExitUserError terminates execution with a user error carrying the contract's
// own message. It does not consume gas, matching runtime.SignalUserError.
func ExitUserError(message string) *EarlyExit {
	return &EarlyExit{
		Code:          vmcommon.UserError,
		Message:       message,
		ConsumeAllGas: false,
		Cause:         ErrSignalError,
	}
}

// ExitOutOfGas terminates execution with an out-of-gas breakpoint only, without
// touching the output state and without consuming the remaining gas. This is the
// legacy async-call behaviour.
func ExitOutOfGas() *EarlyExit {
	return &EarlyExit{
		Code:          vmcommon.OutOfGas,
		Message:       ErrNotEnoughGas.Error(),
		ConsumeAllGas: false,
		Cause:         ErrNotEnoughGas,
	}
}
```

> `ExitUserError` sets `Cause: ErrSignalError` so that the trace entry added by
> `runtime.SignalUserError` is not duplicated — see §5.4 for why the `Cause` of a user error
> is never passed to `AddError` by `applyEarlyExit`.

**(b) `executor/vmHooksErrorHandler.go`** — package `executor`, new file.

`executor/vmHooks.go` is generated and marked DO NOT EDIT, so this interface lives in its own
hand-written file. The generated `VMHooks` interface will embed it (§4.3).

```go
package executor

// VMHooksErrorHandler applies the side effects of an error returned by a VM hook.
// It is implemented by the VM, and called only by generated hook wrappers.
type VMHooksErrorHandler interface {
	// HandleHookError applies the failure side effects for err. It is a no-op
	// when err is nil.
	HandleHookError(err error)
}
```

**(c) `vmhost/vmhooks/earlyExit.go`** — package `vmhooks`, new file.

```go
package vmhooks

import (
	"errors"

	vmcommon "github.com/multiversx/mx-chain-vm-common-go"
	"github.com/multiversx/mx-chain-vm-go/vmhost"
)

// HandleHookError applies the side effects of an error returned by a VM hook.
// Called only from generated hook wrappers.
func (context *VMHooksImpl) HandleHookError(err error) {
	if err == nil {
		return
	}
	applyEarlyExit(context.host, resolveEarlyExit(err))
}

// resolveEarlyExit classifies a hook error into the termination it requires.
// See the contract in the migration document: exactly one call site, never nil in, never nil out.
func resolveEarlyExit(err error) *vmhost.EarlyExit {
	var earlyExit *vmhost.EarlyExit
	if errors.As(err, &earlyExit) {
		return earlyExit
	}

	// The default: fail execution and burn the remaining gas. This is what the
	// 800+ plain-error failure sites used to do via FailExecution.
	return &vmhost.EarlyExit{
		Code:          vmcommon.ExecutionFailed,
		Message:       err.Error(),
		ConsumeAllGas: true,
		Cause:         err,
	}
}

// applyEarlyExit performs the termination. Each branch is exactly one of the three
// effect routines that exist today; no new behaviour is introduced here.
func applyEarlyExit(host vmhost.VMHost, earlyExit *vmhost.EarlyExit) {
	if earlyExit.ConsumeAllGas {
		metering := host.Metering()
		_ = metering.UseGasBounded(metering.GasLeft())
	}

	runtime := host.Runtime()
	switch earlyExit.Code {
	case vmcommon.UserError:
		runtime.SignalUserError(earlyExit.Message)
	case vmcommon.OutOfGas:
		runtime.SetRuntimeBreakpointValue(vmhost.BreakpointOutOfGas)
	default:
		runtime.FailExecution(earlyExit.Cause)
	}
}
```

Check the equivalence before you go further — it is the whole justification for the migration
being a no-op:

| today | after |
|---|---|
| `FailExecution(host, err)` = burn all gas, then `runtime.FailExecution(err)` | `resolveEarlyExit` → `{ExecutionFailed, burn}` → burn, then `runtime.FailExecution(cause)` |
| `runtime.SignalUserError(msg)`, no burn | `ExitUserError(msg)` → `{UserError, no burn}` → `runtime.SignalUserError(msg)` |
| `runtime.SetRuntimeBreakpointValue(BreakpointOutOfGas)`, no burn, no output change | `ExitOutOfGas()` → `{OutOfGas, no burn}` → `runtime.SetRuntimeBreakpointValue(BreakpointOutOfGas)` |

---

## 4. How `resolveEarlyExit` must be used

This is the part most likely to be got wrong. Read it before migrating any hook.

### 4.1 One call site, and it is generated-code-only

`resolveEarlyExit` is called from **exactly one place**: `(*VMHooksImpl).HandleHookError`.
`HandleHookError` is called from **exactly one kind of place**: a generated hook wrapper.

**Never** call `resolveEarlyExit` — or `applyEarlyExit`, or `HandleHookError` — from:

- a hook body,
- a helper called by a hook,
- a `…WithHost` or `…WithTypedArgs` function,
- a context (`vmhost/contexts/*.go`),
- `vmhost/hostCore/*.go`.

It is unexported specifically to make the second and fourth cases impossible from outside the
package, and you must not export it. If you find yourself wanting to call it from a hook body,
the hook should be returning an error instead.

### 4.2 It never receives nil and never returns nil

The `err == nil` check belongs in the **caller** (`HandleHookError`), not in the resolver. This
differs from the old `FailExecution`, which started with `if err == nil { return }`.

Two consequences:

- `resolveEarlyExit` may dereference/inspect `err` unconditionally.
- Generated wrappers must guard with `if err != nil` before calling `HandleHookError`. The
  wrapper's guard and `HandleHookError`'s guard are both present on purpose — the wrapper's
  guard keeps the hot path free of a call, `HandleHookError`'s guard makes the exported
  method safe for the hand-written callers in §7.

### 4.3 It is pass-through for errors that already carry an `EarlyExit`

The `errors.As` branch returns the existing `*EarlyExit` **unchanged**: no re-classification,
no re-wrapping, no message rewriting, no merging of flags. This is what allows a hook (or a
helper several frames down) to declare its own flavour once and have that survive arbitrary
`fmt.Errorf("…: %w", err)` wrapping on the way out.

Corollary: if a hook returns `fmt.Errorf("context: %w", vmhost.ExitUserError("boom"))`, the
resolved message is `"boom"`, not `"context: boom"`. If the added context matters, construct
the `EarlyExit` with the full message instead of wrapping it.

### 4.4 The default branch is the only place the default lives

The fallback `{ExecutionFailed, err.Error(), ConsumeAllGas: true, Cause: err}` governs all
~833 migrated failure sites. `ConsumeAllGas: true` is the default precisely because "consume
all gas on failure" is what almost every hook does today; the exceptions opt out by returning
an `EarlyExit`. Do not change this default as part of the migration.

### 4.5 It must not duplicate what `runtime.FailExecution` already does

`runtime.FailExecution` (`vmhost/contexts/runtime.go:595-612`) already:

- sets `ReturnCode = vmcommon.ExecutionFailed`,
- sets the return message from `err.Error()`,
- calls `AddError(err)`,
- and **upgrades the breakpoint to `BreakpointOutOfGas` when `errors.Is(err, vmhost.ErrNotEnoughGas)`**,
  while leaving `ReturnCode` at `ExecutionFailed`.

Do **not** replicate that last rule in `resolveEarlyExit`. It is tempting to write
`if errors.Is(err, vmhost.ErrNotEnoughGas) { Code = vmcommon.OutOfGas }`, but that would route
the error to the `OutOfGas` branch of `applyEarlyExit`, which sets only the breakpoint and
skips `SetReturnCode` / `SetReturnMessage` / `AddError`. That is a behaviour change. Leave the
upgrade where it is.

Likewise, `applyEarlyExit`'s `UserError` branch must not call `AddError(earlyExit.Cause)`:
`runtime.SignalUserError` already adds `errors.New(message)` itself, and adding the cause too
would put an extra entry in the error trace.

### 4.6 Summary table

| question | answer |
|---|---|
| who calls it | `(*VMHooksImpl).HandleHookError`, nothing else |
| who calls that | generated wrappers, plus the hand-written shims in §7 |
| nil error | rejected by the caller; never reaches the resolver |
| nil result | impossible; always returns a non-nil `*EarlyExit` |
| already an `EarlyExit` | returned verbatim |
| default flavour | `ExecutionFailed`, message `err.Error()`, consume all gas |
| may a hook call it | no |
| may it be exported | no |

---

## 5. Generator changes

All in `vmhost/vmhooks/generate/`. Entry point is `cmd/eiGenMain.go`, run with
`go generate ./vmhost/vmhooks/...` or `cd vmhost/vmhooks && go run generate/cmd/eiGenMain.go`.

### 5.1 `eiGenModel.go` — add one field

```go
 type EIFunction struct {
 	Name      string
 	Arguments []*EIFunctionArg
 	Result    *EIFunctionResult
+
+	// ReturnsError is true when the hook has a trailing `error` result.
+	// During the migration both shapes must be supported; see the parser.
+	ReturnsError bool
 }
```

`Result` keeps its current meaning: the *value* result, `nil` for a void hook. A trailing
`error` is not a value result and must never be represented as one — it does not cross the
C ABI.

### 5.2 `eiGenParse.go` — accept a trailing `error`

Replace `extractEIFunctionResult` (currently lines 53-71, which rejects more than one result)
with a version that strips a trailing `error` first:

```go
func isErrorType(ty ast.Expr) bool {
	ident, ok := ty.(*ast.Ident)
	return ok && ident.Name == "error"
}

func extractEIFunctionResults(decl *ast.FuncDecl) (*EIFunctionResult, bool, error) {
	if decl.Type.Results == nil {
		return nil, false, nil
	}

	list := decl.Type.Results.List
	returnsError := false
	if n := len(list); n > 0 && isErrorType(list[n-1].Type) {
		returnsError = true
		list = list[:n-1]
	}

	switch len(list) {
	case 0:
		return nil, returnsError, nil
	case 1:
		eiType, err := processType(list[0].Type)
		if err != nil {
			return nil, false, err
		}
		return &EIFunctionResult{Type: eiType}, returnsError, nil
	default:
		return nil, false, fmt.Errorf(
			"too many results in function %s, at most one value result plus a trailing error accepted",
			decl.Name.Name)
	}
}
```

Wire it into `extractEIFunction` and set `ReturnsError` on the `EIFunction`.

**Tolerant during the migration, mandatory at the end.** The parser must accept hooks that do
*not* return an error while you are migrating file by file — that is what keeps the tree
compiling at every step. Only once all 277 are migrated do you add the enforcement, as the
last step of the migration (§7 step 8):

```go
	if !returnsError {
		return nil, fmt.Errorf("VM hook %s must return a trailing error", decl.Name.Name)
	}
```

Do not add it earlier. Do not skip adding it at the end — it is what stops a future hook from
silently reintroducing in-body failure handling.

### 5.3 A shared result-signature helper

Add to `eiGenWriteCgo.go` (next to the other type helpers), used by §5.4 and §5.5:

```go
// goResultSignature renders the Go result list of a hook, including the leading space.
func goResultSignature(funcMetadata *EIFunction, typeName func(EIType) string) string {
	switch {
	case funcMetadata.Result == nil && !funcMetadata.ReturnsError:
		return ""
	case funcMetadata.Result == nil:
		return " error"
	case !funcMetadata.ReturnsError:
		return " " + typeName(funcMetadata.Result.Type)
	default:
		return fmt.Sprintf(" (%s, error)", typeName(funcMetadata.Result.Type))
	}
}
```

### 5.4 `eiGenWriteGoVMHooks.go` — the `VMHooks` interface

Two changes to `WriteEIInterface`:

1. Embed the hand-written error handler in the top-level interface, before the groups:

```go
 	out.WriteString(`
 // VMHooks contains all VM functions that can be called by the executor during SC execution.
 type VMHooks interface {`)
+	out.WriteString("\n\tVMHooksErrorHandler")
 
 	for _, group := range eiMetadata.Groups {
```

2. Replace the per-method result rendering (lines 34-38) with
   `goResultSignature(funcMetadata, vmHooksType)`.

Leave `externResult` alone — it renders the **C** result and must keep ignoring `ReturnsError`.

### 5.5 `eiGenWriteVMHooksWrapper.go` — `WrapperVMHooks`

Change the method rendering to pass both results through, and add a `HandleHookError`
passthrough so `WrapperVMHooks` still satisfies `executor.VMHooks`.

Target output per hook:

```go
// StorageLoad VM hook wrapper
func (w *WrapperVMHooks) StorageLoad(keyOffset executor.MemPtr, keyLength executor.MemLength, dataOffset executor.MemPtr) (int32, error) {
	callInfo := fmt.Sprintf("StorageLoad(%d, %d, %d)", keyOffset, keyLength, dataOffset)
	w.logger.LogVMHookCallBefore(callInfo)
	result, err := w.wrappedVMHooks.StorageLoad(keyOffset, keyLength, dataOffset)
	w.logger.LogVMHookCallAfter(callInfo)
	return result, err
}
```

and for a void hook:

```go
	err := w.wrappedVMHooks.SignalError(messageOffset, messageLength)
	w.logger.LogVMHookCallAfter(callInfo)
	return err
```

Note the logger call stays *between* the inner call and the return: `LogVMHookCallAfter` must
still run on the failure path, as it does today. Do not add early returns.

Append once, after the loop:

```go
	out.WriteString(`
// HandleHookError passthrough
func (w *WrapperVMHooks) HandleHookError(err error) {
	w.wrappedVMHooks.HandleHookError(err)
}
`)
```

### 5.6 `eiGenWriteCgo.go` — the cgo exports

`writeCgoFunctions` (the `// extern …` block) and the `//export` **signatures** are
**unchanged**. Only the bodies of the exported functions change, in `writeGoExports`.

Target output, void hook:

```go
//export w2_signalError
func w2_signalError(context unsafe.Pointer, messageOffset int32, messageLength int32) {
	vmHooks := getVMHooksFromContextRawPtr(context)
	err := vmHooks.SignalError(executor.MemPtr(messageOffset), messageLength)
	if err != nil {
		vmHooks.HandleHookError(err)
	}
}
```

Target output, valued hook:

```go
//export w2_storageLoad
func w2_storageLoad(context unsafe.Pointer, keyOffset int32, keyLength int32, dataOffset int32) int32 {
	vmHooks := getVMHooksFromContextRawPtr(context)
	result, err := vmHooks.StorageLoad(executor.MemPtr(keyOffset), keyLength, executor.MemPtr(dataOffset))
	if err != nil {
		vmHooks.HandleHookError(err)
	}
	return result
}
```

Three rules for this writer:

- **Return the hook's own value on the failure path.** `result` is returned as-is, whatever the
  hook produced alongside the error. Do not substitute `0` or `-1`. Today's hooks return a
  specific value next to each `FailExecution` call (often `-1`, sometimes `0`, sometimes the
  partially-computed value) and that value is written to the wasm stack before the breakpoint
  stops execution. Preserving it is what makes the migration a no-op.
- **No Go pointer may be returned.** Unchanged from today, but worth restating: the return type
  stays a scalar. The error never crosses into C.
- **Keep supporting non-erroring hooks** while the migration is in progress: if
  `!funcMetadata.ReturnsError`, emit exactly today's body.

`WriteWasmer1Cgo` (`eiGenWriteCgo.go:95`) is **dead code** — nothing calls it, and there is no
`wasmer1` package left in the repo. Delete it, along with `writePopulateImports` if that
becomes unreferenced too. Do not spend effort updating it.

### 5.7 Generator files that must NOT change

Verify by diff after regenerating (§8). These writers all consume `Result` for the C/Rust ABI
and must ignore `ReturnsError`:

`eiGenWriteRustVMHooksNames.go`, `eiGenWriteRustVMHooksTrait.go`,
`eiGenWriteRustVMHooksTraitLegacy.go`, `eiGenWriteRustVMHooksTraitAdapter.go`,
`eiGenWriteRustVMHooksSignatures.go`, `eiGenWriteRustCapiVMHooks.go`,
`eiGenWriteRustCapiVMHooksPointers.go`, `eiGenWriteRustWasmerProdImports.go`,
`eiGenWriteRustWasmerExperimentalImports.go`, `eiGenWriteRustVHDispatcherLegacy.go`,
`eiGenWriteRustUtil.go`, `eiGenWriteHeaders.go`, `eiGenWriteGoNames.go`, and every
`*OpcodeCost*` / `*Opcode*` writer.

---

## 6. Migrating the hook bodies

### 6.1 The mechanical rules

Apply these per hook. `⟨v⟩` is whatever value the site returns today.

| # | today | after |
|---|---|---|
| R1 | `context.FailExecution(err)` <br> `return` | `return err` |
| R2 | `context.FailExecution(err)` <br> `return ⟨v⟩` | `return ⟨v⟩, err` |
| R3 | `FailExecution(host, err)` <br> `return` / `return ⟨v⟩` | same as R1 / R2 |
| R4 | `context.FailExecution(vmhost.ErrXyz)` <br> `return ⟨v⟩` | `return ⟨v⟩, vmhost.ErrXyz` |
| R5 | `if err != nil && runtime.UseGasBoundedShouldFailExecution() {` <br>  `context.FailExecution(err)` <br>  `return ⟨v⟩` <br> `}` | `if err != nil && runtime.UseGasBoundedShouldFailExecution() {` <br>  `return ⟨v⟩, err` <br> `}` |
| R6 | `runtime.SetRuntimeBreakpointValue(vmhost.BreakpointOutOfGas)` <br> `return ⟨v⟩` | `return ⟨v⟩, vmhost.ExitOutOfGas()` |
| R7 | `runtime.SignalUserError(msg)` (tail of hook) | `return vmhost.ExitUserError(msg)` |
| R8 | `return ⟨v⟩` (success) | `return ⟨v⟩, nil` |
| R9 | bare `return` (success, void hook) | `return nil` |
| R10 | void hook falling off the end of the body | add explicit `return nil` |

R5 deserves attention: the **condition is unchanged**. When
`UseGasBoundedShouldFailExecution()` is false the error is still dropped and execution
continues — that is today's behaviour and must be preserved. Do not "fix" it into
`return ⟨v⟩, err` unconditionally.

R6 applies to exactly 6 sites (`baseOps.go:1694,1715,1783,1801,1859`, `managedei.go:679`).
These are the "exception" cases: the legacy async call reports out-of-gas without burning the
remaining gas and without touching the output state. After migration, no hook in
`vmhost/vmhooks/` calls `SetRuntimeBreakpointValue` at all.

R7 applies to exactly 2 sites: `SignalError` (`baseOps.go:272`) and `ManagedSignalError`
(`managedei.go:192`). After migration, no hook in `vmhost/vmhooks/` calls `SignalUserError`.

There is 1 direct `runtime.FailExecution(` inside `vmhost/vmhooks/`. Locate it, read the
surrounding code, and migrate it by hand — it is not covered by a rule.

### 6.2 Worked example — `SignalError`

Before (`baseOps.go:252-273`):

```go
func (context *VMHooksImpl) SignalError(messageOffset executor.MemPtr, messageLength executor.MemLength) {
	runtime := context.GetRuntimeContext()
	metering := context.GetMeteringContext()
	metering.StartGasTracing(signalErrorName)

	gasToUse := math.MulUint64(metering.GasSchedule().BaseOperationCost.PersistPerByte, uint64(messageLength))
	gasToUse = math.AddUint64(gasToUse, metering.GasSchedule().BaseOpsAPICost.SignalError)

	err := metering.UseGasBounded(gasToUse)
	if err != nil && runtime.UseGasBoundedShouldFailExecution() {
		context.FailExecution(err)
		return
	}

	message, err := context.MemLoad(messageOffset, messageLength)
	if err != nil {
		context.FailExecution(err)
		return
	}

	runtime.SignalUserError(string(message))
}
```

After:

```go
func (context *VMHooksImpl) SignalError(messageOffset executor.MemPtr, messageLength executor.MemLength) error {
	runtime := context.GetRuntimeContext()
	metering := context.GetMeteringContext()
	metering.StartGasTracing(signalErrorName)

	gasToUse := math.MulUint64(metering.GasSchedule().BaseOperationCost.PersistPerByte, uint64(messageLength))
	gasToUse = math.AddUint64(gasToUse, metering.GasSchedule().BaseOpsAPICost.SignalError)

	err := metering.UseGasBounded(gasToUse)
	if err != nil && runtime.UseGasBoundedShouldFailExecution() {
		return err
	}

	message, err := context.MemLoad(messageOffset, messageLength)
	if err != nil {
		return err
	}

	return vmhost.ExitUserError(string(message))
}
```

Note `runtime` is still used (for `UseGasBoundedShouldFailExecution`), so no unused-variable
fallout here. In other hooks, migrating away the last use of `runtime` / `metering` / `host`
**will** produce unused-variable errors — delete the now-dead local.

### 6.3 Worked example — `AsyncCall` (the R6 case)

The tail of `AsyncCall` (`baseOps.go:1856-1866`):

```go
	err = async.RegisterLegacyAsyncCall(calledSCAddress, data, value)
	if errors.Is(err, vmhost.ErrNotEnoughGas) {
		runtime.SetRuntimeBreakpointValue(vmhost.BreakpointOutOfGas)
		return
	}
	if err != nil {
		context.FailExecution(err)
		return
	}
}
```

becomes:

```go
	err = async.RegisterLegacyAsyncCall(calledSCAddress, data, value)
	if errors.Is(err, vmhost.ErrNotEnoughGas) {
		return vmhost.ExitOutOfGas()
	}
	if err != nil {
		return err
	}
	return nil
}
```

### 6.4 Helpers

Three categories, handle them differently.

**Already error-returning — leave alone.** `vmhost/vmhooks/managedConversions.go` contains no
`FailExecution` calls (the grep hits are all `UseGasBoundedShouldFailExecution`); it already
returns `(T, error)`. `vmhost/vmhooks/eei_helpers.go` has no failure handling at all.

**`…WithHost` / `…WithTypedArgs` helpers that call `FailExecution`.** There are ~40 of these
(`grep -n 'WithHost(\|WithTypedArgs(' vmhost/vmhooks/*.go`). Migrate each to return
`(⟨v⟩, error)` / `error` using the same rules, and have the calling hook propagate:

```go
	result, err := context.StorageLoadWithHost(host, keyOffset, keyLength, dataOffset)
	return result, err
```

or just `return context.StorageLoadWithHost(...)` when the shapes line up.

**External callers of those helpers** (`mock/contracts/`, `*_test.go`, ~115 sites) then get a
second return value. At those sites the caller is *not* a hook, so it must apply the effects
itself — call the exported `FailExecution(host, err)` explicitly:

```go
	result, err := vmhooks.ManagedBufferCopyByteSliceWithHost(host, …)
	if err != nil {
		vmhooks.FailExecution(host, err)
		return
	}
```

This is already the idiom at several of those sites (e.g. `vmhost/hosttest/managedops_test.go:33,37`).

### 6.5 `FailExecution` stays

Keep `vmhooks.FailExecution(host, err)` (`vmHooksImpl.go:106-115`) exported and **unchanged**.
It is still used by 17 call sites in mocks and tests, and by the non-hook callers in §6.4.

Keep `(*VMHooksImpl).FailExecution(err)` (`vmHooksImpl.go:96-98`) too, but expect it to become
unused inside the package. Check at the end whether anything still calls it; if not, decide
then whether to delete it (it is not part of the `executor.VMHooks` interface, so removing it
is safe).

`applyEarlyExit` deliberately does **not** call `FailExecution`; it calls
`runtime.FailExecution` directly, because the gas burn is already done by the `ConsumeAllGas`
branch. Routing it through `FailExecution` would burn twice — harmless
(`UseGasBounded(GasLeft())` is 0 the second time) but misleading. Do not "simplify" it.

---

## 7. Order of work

Each numbered step ends with a compiling tree and its own commit. The trick that makes this
possible is `ReturnsError` being *per function* plus the parser staying tolerant until step 8.

1. **Support code.** Add `vmhost/earlyExit.go`, `executor/vmHooksErrorHandler.go`,
   `vmhost/vmhooks/earlyExit.go` (§3.2). Nothing references the new types yet.
   `go build ./...` passes.

2. **Generator.** Apply §5.1-§5.6. Do not add the mandatory-error validation. Delete
   `WriteWasmer1Cgo`. `go build ./...` passes.

3. **Regenerate, no hooks migrated yet.** Run the generator. Expected Go diff: the
   `VMHooksErrorHandler` line in `executor/vmHooks.go` and the `HandleHookError` passthrough in
   `executor/wrapper/wrapperVMHooks.go`. Everything else byte-identical, including all Rust
   outputs. `go build ./...` and `make test-short` pass. **If the Rust outputs moved, stop and
   fix the generator before continuing.**

4. **Migrate the hook files, smallest first.** For each file: apply §6.1, run the generator,
   `go build ./...`, fix the test/mock call sites for that group, run `make test-short`.
   Order: `manMapOps.go` (5) → `smallIntOps.go` (12) → `bigFloatOps.go` (22) →
   `manBufOps.go` (27) → `cryptoei.go` (41) → `managedei.go` (41) → `bigIntOps.go` (42) →
   `baseOps.go` (87).
   `manMapOps.go` is the calibration run: 5 hooks, 9 failure sites, all int32, no helpers. Get
   the full loop working there before touching anything bigger.
   `baseOps.go` is last because it holds the async/out-of-gas cases (R6), most of the
   `…WithHost` helpers, and `SignalError`.

5. **`…WithHost` / `…WithTypedArgs` helpers** not already done as part of step 4, plus their
   external callers (§6.4).

6. **Remove the duplicate `@autogenerate(VMHooks)`** on `UpgradeContract`
   (`baseOps.go:1516-1517`). Regenerate; the hook count printed by the generator should go
   from 278 to 277 and the generated output should not otherwise change.

7. **Sweep.** Run the §8 greps. Everything must be clean before step 8.

8. **Make the error result mandatory.** Add the validation from §5.2, regenerate, build, test.
   This is the step that locks the invariant in.

9. **Optional cleanup**, separate commit: delete `(*VMHooksImpl).FailExecution` if unused.

---

## 8. Verification

**Build and test**

```sh
cd /Users/andreim/multiversx/chain/mx-chain-vm-go
go build ./...
go vet ./...
make test-short          # fast signal during step 4
VMEXECUTOR="wasmer2" go test ./...   # full run, before declaring done
```

Also run `golangci-lint run` if the repo's config is set up — an `errcheck`-style linter will
flag any error you accidentally drop, which is exactly the class of mistake this migration can
introduce.

**Greps that must come back empty** (run from `mx-chain-vm-go`):

```sh
# no hook sets a breakpoint any more
grep -rn "SetRuntimeBreakpointValue" vmhost/vmhooks/

# no hook signals a user error directly any more
grep -rn "SignalUserError" vmhost/vmhooks/

# no failure side effects inside hook bodies; the only remaining hits
# must be the definition in vmHooksImpl.go and UseGasBoundedShouldFailExecution
grep -rn "FailExecution(" vmhost/vmhooks/*.go | grep -v ShouldFailExecution

# resolveEarlyExit / applyEarlyExit have exactly one caller each
grep -rn "resolveEarlyExit\|applyEarlyExit" .
```

**The Rust outputs must not move.** The generator copies files into `mx-vm-executor-rs` when
`vmhost/vmhooks/generate/cmd/path-to-vm-executor-rs.txt` exists — and it does. After every
regeneration:

```sh
cd /Users/andreim/multiversx/chain/mx-vm-executor-rs && git status --short
```

must show no change to `vm-executor/src/`, `c-api/src/`, `vm-executor-wasmer/src/` or
`vm-executor-experimental/src/`. Same for the `mx-sdk-rs` copies if
`path-to-mx-sdk-rs.txt` is present.

**Spot-check the diff of the generated cgo file.** `wasmer2/wasmer2ImportsCgo.go` should show
changes only inside `//export` function bodies. If any `// extern` line or any `//export`
signature changed, the C ABI moved and something is wrong.

**Behaviour equivalence.** The scenario/JSON tests under `vmhost/hosttest/`,
`vmhost/vmhookstest/` and the scenario runner are the real check. They assert return codes,
return messages and exact gas figures, so a misplaced gas burn or a lost `AddError` will show
up as a diff. Do not accept a "fixed" expectation file without understanding why it moved.

---

## 9. Deliberately deferred

Do not do these now, even though the migration makes them tempting.

- **Return the early-exit code over the FFI.** That is `PLAN-go-ffi.md` Phase 3. Until then
  `HandleHookError` returns nothing and the breakpoint globals still carry the outcome.
- **Fold the `ErrNotEnoughGas` breakpoint upgrade into `resolveEarlyExit`** (see §4.5).
- **Make `ConsumeAllGas` derivable from `Code`.** It is carried explicitly because the two are
  independent in general: a `UserError` keeps its gas while an `ExecutionFailed` burns it, and
  the FFI phase needs the flag separately from the code.
- **Normalise the values returned alongside errors** (the mix of `-1`, `0` and partial values).
  Preserving them is what keeps this migration a no-op; changing them is a separate, testable
  decision.
- **Unify `UseGasBoundedShouldFailExecution` handling.** The flag-gated drop-the-error pattern
  at ~300 sites is odd but load-bearing; leave it.
- **Delete `vmhooks.FailExecution`.** Non-hook callers still need it (§6.5).

---

## 10. Quick reference

| thing | where |
|---|---|
| hook sources | `vmhost/vmhooks/{baseOps,managedei,bigFloatOps,bigIntOps,manBufOps,manMapOps,smallIntOps,cryptoei}.go` |
| generator | `vmhost/vmhooks/generate/`, entry `generate/cmd/eiGenMain.go` |
| run generator | `cd vmhost/vmhooks && go run generate/cmd/eiGenMain.go` |
| generated `VMHooks` interface | `executor/vmHooks.go` |
| generated logging wrapper | `executor/wrapper/wrapperVMHooks.go` |
| generated cgo exports | `wasmer2/wasmer2ImportsCgo.go` |
| cgo context helper | `wasmer2/wasmer2ImportsCgoHelper.go:9` |
| `FailExecution` (hooks) | `vmhost/vmhooks/vmHooksImpl.go:106` |
| `runtime.FailExecution` | `vmhost/contexts/runtime.go:595` |
| `runtime.SignalUserError` | `vmhost/contexts/runtime.go:616` |
| breakpoint → error mapping | `vmhost/hostCore/breakpoints.go:26` |
| new: `EarlyExit` | `vmhost/earlyExit.go` |
| new: `VMHooksErrorHandler` | `executor/vmHooksErrorHandler.go` |
| new: `resolveEarlyExit` | `vmhost/vmhooks/earlyExit.go` |
