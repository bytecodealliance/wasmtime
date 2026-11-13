# Results: proposals for reducing host↔guest call overhead

Each proposal in `proposals.md` was implemented, in several variants where
that seemed useful, and measured with the `wasi-http/p3_http_proxy` benchmark.
Every attempt is on its own branch.

## Summary

| Proposal | Best variant | Change vs. baseline |
|---|---|---|
| 1. Inline the deferred host `CallContext` | — | **none measurable** (best: −0.3%, within noise) |
| 2. Skip context-slot save/restore for sync host calls | `abk/opt2a-lazy-context-slots` | **−6%** |
| 3. Typed tables for async runtime objects | `abk/opt3f-typed-table-boxed-over-48` | **−10.5%** |
| 2 + 3 + inline `TypeId` in `ResourceTable` | `abk/opt-combined` | **−12%** (17.4 µs → 15.3 µs) |

Proposals 2 and 3 overlap: combined they give about −12%, not −16%. Proposal
2's win came from avoiding two generic `ResourceTable` lookups per host call,
and proposal 3 makes those same lookups cheap.

The biggest lesson is that the generic `ResourceTable` costs much more than
the profile suggested. The profile put roughly 1–2% of samples in any single
table function, yet replacing the table saved about 10%. That cost is spread
across many callers as inlined code, pointer chasing through `Box<dyn Any>`,
virtual `type_id` calls, and copying large entries. A flat profile doesn't
show it in one place.

## Method

- **Baseline:** `abk/proxy-profiling` at `cf9647e0a2`, about **17.4 µs** per
  request.
- **Builds:** each branch's benchmark binary is built with
  `cargo bench -p wasmtime-wasi-http --bench p3_http_proxy --no-run` (release
  profile) and copied aside.
- **Runs:** binaries run interleaved (A, B, C, A, B, C, …), 3–4 rounds each,
  pinned to one core with `taskset -c 3`, using Criterion with 2 s of warm-up
  and 6 s of measurement. The reported number is the median of the
  per-run Criterion means.
- **Noise:** two copies of the same baseline binary differed by 0.1%, with a
  run-to-run spread of about ±0.3%. Different builds can move by up to about
  ±0.5% from code layout alone. So differences under about 1% are treated
  as noise.
- **Tests:** every branch passes `cargo test --test wast -- component-model`
  (400 tests), `cargo test --test all -- component_model` (264), and
  `cargo test -p wasmtime-wasi-http --test all p3` (31). The typed-table
  branches also pass `cargo test -p wasmtime --lib concurrent`.

All the per-run numbers in the tables below come from interleaved runs that
included the baseline. Percentages are relative to the baseline in the same
session.

## Proposal 1: inline the `CallContext` for deferred host scopes

**Result: no measurable effect.**

| Branch | Change | vs. baseline |
|---|---|---|
| `abk/opt1a-boxed-call-context` | `CurrentScope::DeferredHost(NonNull<CallContext>)` points at a heap-allocated `BoxedCallContext`, which keeps its address when the host task is materialized. A spare box is kept for reuse. | **+0.9%** (slower) |
| `abk/opt1b-boxed-call-context-recycled` | 1a, plus deleted host tasks return their box to the spare slot | −0.2% / −0.3% (noise) |
| `abk/opt1c-scope-once-per-call` | 1b, plus `call_sync_lower` passes the scope it lifted with into the `LowerContext`. That skips the `StoreOpaque::current_scope` lookup in `validate_scope_exit` and when lowering results. | +0.3% (noise) |

Options considered for holding the context:
- **A `&mut` reference:** not possible. `CurrentScope` is `Copy`, and
  `ResourceTables` already holds `&mut ComponentTaskState`, which owns the
  context.
- **`Arc`/`Rc`:** a reference-count change per scope would cost more than the
  lookup it replaces.
- **A stable heap allocation plus a raw pointer:** the only workable option.
  It's a hand-rolled `NonNull` owner rather than a `Box`, because moving a
  `Box` invalidates raw pointers into it under Stacked Borrows.

Findings:
- **The lookup being removed was already cheap.** It's a `match` on
  `ComponentTaskState` plus an `Option` check. The pointer still has to be
  produced somewhere, in `StoreOpaque::current_scope`, so the work moves
  rather than disappears.
- **1a was slower because of allocation churn.** A materialized host task
  (`client.send`, `consume-body`, etc.) took the deferred context's box with
  it and freed it on deletion, so the next deferred call allocated again.
  Recycling the box (1b) removed the slowdown but gained nothing.
- **Looking up the scope once per call (1c) also gained nothing.** The
  `current_scope` self time in the profile (~1.3%) doesn't correspond to
  avoidable work.
- **Downsides:** this adds `unsafe` code, a raw pointer in `CurrentScope`
  (which makes it `!Send`), and an allocation-failure path in
  `materialize_host_task` that can lose the context (marked `FIXME`).
  **Not worth pursuing.**

## Proposal 2: optimized `set_thread` for sync host calls

**Result: about −6%.**

| Branch | Change | vs. baseline |
|---|---|---|
| `abk/opt2a-lazy-context-slots` | `host_task_create` uses `set_thread_deferred_host`, which leaves the calling guest thread's `context.get/set` slots in the `VMStoreContext` and records that thread as `lazy_context_owner`. `set_thread` saves those slots only if a *different* guest thread is switched to first. When the caller is resumed, it skips the restore entirely. | **−6.0%** (confirmed in reverse order: −6.0%) |
| `abk/opt2b-streamlined-host-task-enter-exit` | 2a, plus forcing the current thread only once in `host_task_create`, and a fast path in `host_task_delete` for unmaterialized tasks that bypasses `set_thread` | −6.8% vs. 2a's −6.7%: no additional gain |

How 2a stays correct:
- Host code can't run `context.get`/`set`. Only JIT code for the *current
  guest* thread touches the slots. Any switch to a guest thread goes through
  `set_thread`, which saves the slots for the lazy owner first.
- The slots are only ever read or written in `set_thread`.
- If the owner thread is deleted while it's still the lazy owner
  (`cleanup_thread`), the field is cleared.
- The proposal said "sync context only". This implementation applies to every
  deferred host call (sync or async lowered), because the lazy save makes it
  safe whether or not the host call blocks or switches threads.

Findings:
- **The 6% is far more than the ~1.4% of samples `set_thread` had in the
  profile.** The saving isn't really the copy of the two context slots.
  It's the two `ConcurrentState::get_mut(GuestThread)` calls per host call,
  one to save and one to restore, each going through the generic
  `ResourceTable`: bounds check, `Box` pointer chase, and a virtual
  `Any::type_id` call.
- **Evidence:** on top of the typed table from proposal 3, which makes those
  lookups cheap, 2a adds only about 1.5% (`abk/opt-combined-no-3a` −11.8%
  vs. 3f −10.2% in the same session).
- **2b's extra streamlining** (fewer `current_thread()` checks, skipping
  `set_thread`'s generic path) is in the noise. Those checks are cheap.

## Proposal 3: typed tables for async runtime objects

**Result: up to about −10.5%, depending on how big each table slot is.**

`ConcurrentState` stored every object in a generic `ResourceTable`: a
`Box<dyn Any + Send>` per entry, plus a `BTreeSet<u32>` of children and an
optional parent. The object types are `GuestTask`, `GuestThread`, `HostTask`,
`WaitableSet`, `TransmitState`, `TransmitHandle`, `ErrorContextState`,
`Option<WorkItem>`, and, with `task-group-hook`, `TaskGroup`.

| Branch | Change | Slot payload size | vs. baseline |
|---|---|---|---|
| `abk/opt3a-resource-table-inline-typeid` | Keep `ResourceTable`, but store each entry's `TypeId` inline so typed get/delete compare it directly instead of making a virtual `type_id` call. This also speeds up the wasi host's own `ResourceTable`. | (unchanged) | −2.4% / −2.9% |
| `abk/opt3b-typed-concurrent-table` | New `ConcurrentTable`: one `Vec` of `enum Item { GuestTask(..), … }` slots, stored inline. Parent/child tracking becomes a child count plus a parent index. | 288 B (`GuestTask`) | −5.2% / −5.0% |
| `abk/opt3c-typed-table-boxed-large` | 3b, with `GuestTask` boxed | 160 B (`TransmitState`) | −3.7% |
| `abk/opt3d-typed-table-boxed-all-large` | 3b, with `GuestTask`, `GuestThread`, `TransmitState` and `WorkItem` boxed | 96 B (`HostTask`) | −6.2% / −6.4% |
| `abk/opt3e-typed-table-boxed-over-64` | 3d, plus `HostTask` boxed | 64 B (`WaitableSet`) | −8.1% / −7.9% |
| `abk/opt3f-typed-table-boxed-over-48` | 3e, plus `WaitableSet` and `ErrorContextState` boxed; only `TransmitHandle` (and `TaskGroup`) inline | 44 B (`TransmitHandle`) | **−10.5% / −10.0% / −10.8% / −10.2%** |
| `abk/opt3g-typed-table-boxed-all` | Everything boxed: only the enum tag and pointer are inline | 8 B | −7.0% |
| `abk/opt3h-per-type-slabs` | A separate exactly-sized `Vec` slab per type, with nothing boxed. IDs are unique only within a type. | exact per type | −8.6% |

Findings:
- **Getting rid of `Box<dyn Any>` and the `BTreeSet` is worth about 5% on its
  own (3b).** The inline `TypeId` in 3a recovers about half of that while
  keeping the generic table.
- **Slot size matters a lot.** With one enum, every slot is as large as the
  largest variant. Every push and delete moves a whole slot (the `Slot` is
  built and then copied into the `Vec`; `mem::replace` on delete), and
  smaller slots also use the cache better. Shrinking slots from 288 B to
  44 B by boxing the large, rarely created types took the gain from −5% to
  −10.5%.
- **Allocation still matters.** Boxing everything (3g) lost about 3.5 points
  compared to 3f. `TransmitHandle` is created and dropped several times per
  request (two per future/stream), so it's worth keeping inline. The best
  layout keeps small, frequently created objects inline and boxes large,
  rarer ones.
- **Per-type slabs (3h) are slightly worse than 3f** (−8.6% vs. −10.5%).
  Large types like `GuestTask` (288 B) and `TransmitState` (160 B) are again
  moved inline on push and delete. A per-type slab that boxes only the large
  types would probably match 3f, but wasn't tried.
- **3c is slower than 3b, the one result that looks inconsistent.** Boxing
  only `GuestTask` adds an allocation and a pointer chase on every
  `GuestTask` access (one of the hottest types), while slots are still
  160 B. The benefit only shows once slots are much smaller.

API and behavior changes in 3b–3h:
- `Store::concurrent_resource_table()` / `StoreContextMut::concurrent_resource_table()`
  now return `Option<&mut ConcurrentTable>` instead of
  `Option<&mut ResourceTable>`. This is a public API change.
  `ConcurrentTable` keeps `max_capacity`/`set_max_capacity`, so the
  `wasmtime-wast` usage still compiles.
- `ConcurrentTable::delete` now checks the type *before* freeing the slot.
  `ResourceTable` freed the entry and then reported `WrongType`.
- In 3h only, IDs can repeat across types. Nothing in the tests relied on
  IDs being unique across types, but this hasn't been audited.
- In 3h, `max_capacity` is enforced on the live entry count. `ResourceTable`
  enforced it on the backing `Vec`'s length when no free slots were left.

## Combinations

| Branch | Contents | vs. baseline |
|---|---|---|
| `abk/opt-combined` | 3f + 2a + 3a | **−11.7% / −12.3%** (17.4 µs → 15.3 µs) |
| `abk/opt-combined-no-3a` | 3f + 2a | −11.8% |

- 2a on top of 3f adds about 1.5%, much less than 2a's 6% alone, as explained
  under proposal 2.
- 3a adds about 0.5% on top of 3f + 2a, within noise. Once the concurrent
  state no longer uses `ResourceTable`, 3a only affects the wasi host's
  resource lookups.

## Recommendations

1. **Pursue proposal 3 (typed concurrent table).** It's the largest win and
   it's a self-contained change.
   - The 3f layout (small, frequently created objects inline; large ones
     boxed) is the best measured. A cleaner design might be per-type slabs
     with the large types boxed, which should match 3f while keeping each type
     in its own storage.
   - Decide what to do about the public
     `Store::concurrent_resource_table` API.
2. **Proposal 2 (lazy context slots)** is a small, local change worth about
   1.5% on top of 3, or about 6% alone. The extra streamlining in 2b isn't
   needed.
3. **3a (inline `TypeId` in `ResourceTable`)** is a tiny, low-risk change
   that helps any embedder using `ResourceTable` heavily. Its benefit to this
   benchmark after 3f is marginal.
4. **Drop proposal 1.** It adds `unsafe` code for no measurable gain.

Follow-ups suggested by these results:
- The sensitivity to slot size suggests the remaining `memcpy` (~6% in the
  original profile) from moving large values is worth attacking more
  generally. Examples are `TransmitState::new`, `GuestTask::new`, and
  `into_http_with_getter`.
- The host's own `ResourceTable` (used by wasi-http for `Request`, `Fields`,
  etc.) has the same `Box<dyn Any>` + `BTreeSet` design. A similar
  redesign, or at least 3a, may help host-implementation-heavy workloads.
