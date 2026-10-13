# Resetting a Wasmer 7 instance in place, using the middlewares

**Status:** design only, not built. The shipped implementation re-instantiates the module
instead — see `ExperimentalInstance::reset`. This document exists so that the faster version
can be picked up without re-deriving it, and so the decision not to build it yet is recorded
with its reasons rather than as a shrug.

**Repo:** `mx-vm-executor-rs`, crate `vm-executor-experimental`.

## Why this document exists

Wasmer 7 has no `Instance::reset`. The Wasmer 2.2 fork does, because MultiversX added it
(`lib/vm/src/instance/reset.rs` is not upstream), and that is what makes warm instances work
on the 2.2 path today.

The obvious conclusion — "so Wasmer 7 cannot reset, and getting it means forking Wasmer 7" —
is wrong, and it is worth being precise about why, because it is the kind of conclusion that
hardens into received wisdom. Wasmer 7 *can* reset. Everything needed is reachable from
inside our own middlewares, which already do the one thing that looks impossible from the
outside: create globals and export them so the host can reach them at runtime.

What follows is how. The shipped implementation does something simpler and slower, and the
last section says why that was the right call for now.

## What a reset has to restore

A reset has to leave the instance in the state it had immediately after instantiation.
Enumerated against what a contract can actually touch under this VM's rules:

| State | Must restore? | Why |
| --- | --- | --- |
| Linear memory contents | **yes** | any store writes it |
| Linear memory length | **yes** | `memory.grow` is allowed and metered |
| Mutable globals (contract's own) | **yes** | `global.set` is allowed |
| Immutable globals | no | nothing can write them |
| Metering / breakpoint globals | **yes** | written on every call |
| Tables | no, under OpcodeVersion **V2** | `TableSet`/`TableGrow`/`TableInit` are all `Cost::Illegal` in V2 |
| Passive data segments | no | `memory.init` and `data.drop` are `Cost::Illegal` in **both** V1 and V2 |
| Passive element segments | no | `elem.drop` is `Cost::Illegal` in both; nothing can retire a segment |

Two of these deserve their own note.

**Passive segments are inert under this VM.** Since neither `data.drop` nor `elem.drop` is
ever legal, the per-instance "which segments are still live" bookkeeping cannot change, so
there is nothing to restore. The 2.2 fork's `reset_passive_elements` is, in this VM, a no-op
that re-runs an initialization-only function — it even carries
`debug_assert!(passive_elements.is_empty(), "should only be called once, at initialization
time")`, inert in release builds but a clear signal of intent.

**Tables are a live gap on the 2.2 path, under V1.** V1 whitelists `TableSet`, `TableGrow`
and `TableInit`, all of which mutate a table, and the fork's `InstanceHandle::reset` restores
passive elements, globals and memories — *not* tables. So under V1 a contract that mutates its
own table and is then served a warm instance observes its own previous mutation. V2 drops all
five table opcodes, which closes it. The practical exposure is small, because the stated
rationale for dropping them in V2 is that no contract built with current tooling can emit
them, so this is hand-written wasm in pre-V2 epochs. It is recorded here because a design for
"reset in place" has to decide what it covers, and because it is an existing asymmetry between
what the whitelist permits and what reset restores.

## The part that looks impossible, and is not

A host holding a `wasmer::Instance` can reach globals only through the module's **exports**:
`instance.exports.get_global(name)`. A contract's own globals are typically not exported —
`test/contracts/wasmbacking/multiple-mutable` declares four mutable globals and exports none
of them — so from outside, they are unreachable and unresettable.

But we are not outside. `ModuleMiddleware::transform_module_info(&self, &mut ModuleInfo)` runs
during compilation with full mutable access to the module's metadata, and it is already used
for exactly this: `we_helpers::create_global_index` creates a global, pushes its initializer,
and inserts an entry into `module_info.exports`. That is the only reason
`set_points_used(instance, store, value)` can work at all.

So the capability already exists in production. It simply has not been pointed at the
contract's own globals.

`ModuleInfo` carries everything needed:

| Field | Use |
| --- | --- |
| `globals: PrimaryMap<GlobalIndex, GlobalType>` | which globals exist, and whether each is mutable |
| `global_initializers: PrimaryMap<LocalGlobalIndex, GlobalInit>` | **the values to restore to** |
| `exports: IndexMap<String, ExportIndex>` | where to add the handles the host will use |
| `num_imported_globals: usize` | to tell local globals from imported ones |
| `custom_sections` / `custom_sections_data` | an alternative place to stash data, see below |

## The design

### 1. A `ResettableGlobals` middleware

In `transform_module_info`, walk the local globals and export every **mutable** one under a
reserved name:

```rust
const RESET_GLOBAL_PREFIX: &str = "mx_resettable_global_";

fn transform_module_info(&self, module_info: &mut ModuleInfo) -> Result<(), MiddlewareError> {
    let first_local = module_info.num_imported_globals;
    let mut exported = Vec::new();

    for (index, global_type) in module_info.globals.iter().skip(first_local) {
        if global_type.mutability.is_const() {
            continue; // nothing can write it, so nothing to restore
        }
        let local = LocalGlobalIndex::new(index.as_u32() as usize - first_local);
        let name = format!("{RESET_GLOBAL_PREFIX}{}", local.as_u32());
        module_info.exports.insert(name.clone(), ExportIndex::Global(index));
        exported.push((name, module_info.global_initializers[local]));
    }

    *self.resettable.lock().unwrap() = Some(exported);
    Ok(())
}
```

Skipping immutable globals is not an optimization detail — it is what keeps the export list
small for typical contracts, which have one mutable stack pointer and a handful of constants.

Imported globals are skipped too: they are not local, have no initializer here, and this VM
provides none.

### 2. Carry the initializers to where reset runs

The middleware learns the initializers at compile time; reset needs them at run time, and in
between the module may have been serialized into the compiled-code cache and reloaded in
another process. Two options:

- **Recover them at instantiation.** `Module::info()` is public and returns the same
  `ModuleInfo`, including `global_initializers` and `exports`. So nothing has to be carried:
  the instance can rebuild the (name, initial value) list from the module it was instantiated
  from, by scanning exports whose name starts with the reserved prefix. **This is the simpler
  option and the recommended one** — it works identically on the fresh-compile and
  from-cache paths, with no extra state and no serialization concerns.
- **Stash them in a custom section.** `custom_sections` and `custom_sections_data` are both
  `pub` on `ModuleInfo`, and `Module::custom_sections(name)` is public for reading, so a
  middleware could write an encoded table that survives serialization. Only worth it if the
  recovery scan ever proves too slow, which at a handful of globals it will not.

### 3. Restore memory from a snapshot

The memory half has no equivalent trick, because active data initializers are **not** in
`ModuleInfo` — only `passive_data` is — and `Module` in Wasmer 7.5 exposes `imports()`,
`exports()`, `custom_sections()` and `info()`, with no `artifact()`. The fork reaches them via
`self.module.artifact().data_initializers()`; that door is closed here.

It does not need to be open. The initial memory image is simply *what the memory contains
immediately after instantiation, before any call runs*, so capture it there:

```rust
// at instantiation, before the first call
let view = memory.view(&store);
let initial_pages = view.size();
let mut image = vec![0u8; view.data_size() as usize];
view.read(0, &mut image)?;
```

and restore it with `Memory::reset` → `grow(initial_pages)` → `MemoryView::write(0, &image)`.
`MemoryView::read`/`write`, `Memory::grow` and `Memory::reset` are all public in 7.5.

**The image belongs to the module, not the instance.** Every instance of a module starts
identical, so it should hang off the compiled module behind an `Arc` — one copy per module,
shared by every instance and every reset. For a two-page contract that is 128 KB once, rather
than 128 KB per cached instance.

A denser variant stores only the non-zero extents (the data segments' ranges), at the cost of
a sparse write loop instead of one `write`. Not worth it until a profile says so.

### 4. What reset then does

```
for (name, initial) in resettable_globals:
    instance.exports.get_global(name).set(store, initial)
set metering + breakpoint globals          (already possible today)
memory.reset(store)                        (to zero length)
memory.grow(store, initial_pages)
memory.view(store).write(0, &module.initial_image)
```

No table work and no passive-segment work, per the table above.

## One step that is not yet verified

`Memory::reset` is documented as *"Resets the memory back to zero length"*. Whether a
subsequent `grow` back to the declared minimum is well-behaved — and whether a memory sitting
at zero length while its `MemoryType` declares a non-zero minimum is a state Wasmer is happy
with — has not been tested. **Probe this first.** If it misbehaves, the fallback is to skip
the shrink and only rewrite the first `initial_pages` worth of memory, accepting that a
contract which grew its memory keeps the larger allocation across a reset. That is a
divergence from the 2.2 fork's behaviour, which does shrink, and would need its own
justification.

## Risks to handle

**Reserved-name collisions are already a latent bug.** `create_global_index` inserts into
`module_info.exports` with no collision check, so a contract that exports a global named
`metering_points_limit` silently displaces the middleware's own export today. Adding a family
of `mx_resettable_global_*` names widens that surface. The fix is a compile-time guard that
rejects a module already exporting any reserved name, and it is worth doing **independently of
this design**, since the existing exposure does not depend on it.

**Extra exports change the artifact.** More entries in `ModuleInfo::exports` means different
serialized bytes, so every entry in the compiled-code cache is invalidated and must be purged
on rollout. Execution semantics and gas are unaffected — exports cost nothing and are not
reachable by the contract — so this is a cache-migration concern rather than a consensus one,
but it has to be sequenced deliberately.

**Nothing new becomes visible to the contract.** A contract addresses globals by index, not by
export name, so exporting a global grants the contract no access it did not already have. The
`ProtectedGlobals` middleware is unaffected: it blocks `global.set` on the middleware's own
indices, which these are not.

## Why this was not built

The shipped `reset` re-instantiates the module into a fresh store, reusing the engine. It is
slower — a full instantiation, including rebuilding the import object, instead of a few global
writes and one memory copy — and it is correct for exactly the same reason: instantiating a
module *is* the operation that produces a fresh instance. It is also strictly more thorough
than the design above and than the 2.2 fork, since tables and passive-element state come back
too, without anyone having to reason about whether they could have changed.

The measured case for going faster is weak. Instantiating an already-compiled module costs
~30–50 µs against ~6 ms to compile it, so the whole warm-instance tier is worth under 1% over
a compiled-code cache hit — and the in-place design optimizes a fraction of that 1%. Against
that sits a new middleware, a reserved-name guard, a per-module memory image, an artifact
migration, and an unverified assumption about `Memory::reset`.

So: build this when there is a reason beyond speed — per-instance host state that must survive
reuse, or a profile that shows instantiation actually mattering — and not before. The point of
writing it down is that the question should be reopened on evidence, not re-derived from
scratch, and certainly not dismissed with "Wasmer 7 can't reset", which is false.
