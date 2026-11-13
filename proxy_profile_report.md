# Profile report: `p3_http_proxy` benchmark

Profile of the `wasi-http/p3_http_proxy` Criterion benchmark
(`crates/wasi-http/benches/p3_http_proxy.rs`), recorded with samply using the
`profiling` cargo profile, a release-built guest with line tables, and
`ProfilingStrategy::PerfMap` for JIT code.

## Summary

Most of the time goes to the machinery for crossing between guest and host,
not to work done on either side. Guest Wasm code is only about 12% of samples.
Within guest→host calls, Wasmtime's call handling, resource tables and async
bookkeeping take about 48 percentage points, against roughly 10 points of
actual wasi-http work. There is real room to reduce call overhead in both
directions, and it is concentrated in a few places.

## Method

- 10,111 samples on the benchmark thread.
- Host frames were resolved against the benchmark binary's full symbol table,
  keeping inlined frames. JIT frames use the perf-map names stored in the
  profile.
- Samples are grouped by matching frame names, so category boundaries are
  approximate.
- At about 17.4 µs per request, 1% of samples ≈ 175 ns per request.
- Per-call costs below assume the proxy makes each listed call once per
  request. That matches the guest code, but calls were not counted directly.

## Where the time goes

| Region | Share |
|---|---|
| Inside host calls made by the guest | **64.1%** |
| Host code with no guest frame on the stack (event loop, starting the export call, harness) | 23.9% |
| Guest code (URL parsing ~3%, guest allocator ~2%, wit-bindgen async runtime ~2%) | 12.0% |

Calling back into the guest's `cabi_realloc` during lowering is negligible
(0.3%).

## Guest → host calls (64%)

### By layer

| Layer | Share |
|---|---|
| Host-call glue: entry, lift/lower, scope checks | 15.5% |
| Resource and handle tables | 15.7% |
| Async futures/streams (transmit state) | 10.1% |
| Async tasks and threads (`host_task_create`, `set_thread`, …) | 7.0% |
| **Actual wasi-http implementation** | **10.2%** |
| Other (mostly `memcpy`) | 5.4% |

### By call

| Call | Total | Impl | Notes |
|---|---|---|---|
| `client.send` | 13.0% | 5.9% | The most real work: oneshot channels, `AbortOnDropJoinHandle`, `send_dummy_io` |
| `request.new` | 10.5% | 0.9% | 3.8% resource tables, 3.6% future/stream setup |
| `request.consume-body` | 7.8% | 0.8% | Creates a `stream<u8>` plus a trailers future |
| `fields.copy-all` | 3.9% | 0.3% | 2.6% glue (lowering a list of strings) |
| `set-path-with-query` / `set-authority` | 3.5% / 3.0% | ~1.1% / 0.4% | URI parsing plus `memcpy` |
| `get-headers`, `set-scheme`, `fields.new`, `set-method`, `get-method` | **1.9–2.8% each** | **≤0.2%** | Almost pure overhead, about **330–490 ns per call** |
| Async built-ins: `future.write`, `task.return`, `future.drop-readable`/`-writable`, `future.new`, `resource.drop` | 11.1% combined | — | All runtime bookkeeping |

### Cost of a trivial host call

The trivial getters and setters show the fixed cost of a host call. For
something like `request.get-method` (a sync-lowered call taking one
`borrow<request>`), nearly all of the ~350 ns goes to the same short list of
functions:

- `StaticHostFn::entrypoint`, then `load_params` and `lower_raw`.
- `host_task_create` and `host_task_delete`, which call `set_thread` twice.
  Each `set_thread` runs a thread-switch handler and saves/restores context
  (`crates/wasmtime/src/runtime/component/concurrent.rs`, `host_task_create`
  and `set_thread`).
- `current_scope`, `component_resource_tables_and_host_resource_data` and
  `options_memory_mut`.
- `resource_lift_borrow`, then `LowerContext::validate_scope_exit` and
  `resource_undo_lend`.
- One malloc/free pair per call that lifts a resource: `resource_lift_borrow`
  allocates when it pushes onto the borrow-tracking `lenders` `Vec`, and
  `validate_scope_exit` frees it. This is a small part of the per-call cost,
  though: allocator time under these two functions is only about 0.36% of
  samples (~60 ns per request across all host calls). See "Reusing the
  `CallContext`" below.

Repeated lookups cost more than allocation does. `current_scope` is 3.3%
inclusive with no allocation at all. It is reached several times per call:
from `LiftContext`, `resource_lift_borrow`, `validate_scope_exit` and
`call_context()`. Each time it goes through `current_thread()` and
`task_state.call_context(scope)`. Similarly, `validate_scope_exit` is 3.3%
inclusive, but only 0.11% of that is `free`. The rest is the scope lookup plus
`table_for_index` → `unwrap_concrete_instance` → `resource_undo_lend`.

| Function | Inclusive | of which malloc/free |
|---|---|---|
| `StaticHostFn` entry (all guest→host calls) | 52.8% | 3.0% |
| `lower_raw` | 7.8% | 0.6% |
| `load_params` | 7.2% | 0.3% |
| `validate_scope_exit` | 3.3% | 0.1% |
| `current_scope` | 3.3% | 0.0% |
| `host_task_create` | 1.9% | 0.0% |
| `component_resource_tables_and_host_resource_data` | 1.8% | 0.0% |
| `resource_lift_borrow` | 1.8% | 0.3% |
| `set_thread` | 1.4% | 0.0% |
| `host_task_delete` | 0.5% | 0.0% |

Top self-time functions across the trivial calls: the `StaticHostFn`
entrypoint (inlined lift/lower), `host_task_create` 0.57%,
`component_resource_tables_and_host_resource_data` 0.54%, `current_scope`
0.51%, `validate_scope_exit` 0.42%, `set_thread` 0.29%,
`resource_lift_borrow` 0.29%, `LowerContext::realloc` 0.22%,
`TypeResourceTable::unwrap_concrete_instance` 0.21%, followed by
`ResourceTable::push_`, `HandleTable::insert` and `resource_undo_lend`.

## Host → guest calls

Host-only time on the main stack is 18.9%, of which about 6.4% is the
benchmark harness itself. The Wasmtime part is about 10–12%, spread over:

- `handle_guest_call` (4.3% inclusive)
- `handle_callback_code` (2.2%)
- `cleanup_thread` (2.0%)
- `prepare_call` and `stage_call`, with a boxed `StagedCall` (about 1.8% + 1.1%)
- `GuestTask::new` (1.0%) and `decrement_task_ref_count` (0.9%)
- Fiber resume/suspend (about 2–3% across `resume_fiber`, `suspend` and
  `FiberFuture::poll`)
- Converting the request/response at the boundary (`Request::from_http`,
  `Response::into_http_with_getter`, `HeaderMap::from(FieldMap)`): about 4.7%

## Costs across all calls

- **`memcpy`, 6%:** spread thinly, mostly from moving large values:
  `into_http_with_getter`, `new_transmit`/`TransmitState::new`,
  `http::request::Builder` chains, and large async state machines.
- **Host malloc/free, about 4%:** much of it is one allocation per table
  entry. `ResourceTable` stores every entry as a `Box<dyn Any + Send>` plus a
  `BTreeSet` of children (`crates/wasmtime/src/runtime/component/resource_table.rs`).
  The async runtime keeps its own `TransmitHandle`, `TransmitState`,
  `GuestTask` and similar objects in this table, so every future or stream
  operation boxes, downcasts and frees. `delete_entry`, `push_` and
  `delete_maybe_debug<TransmitHandle>` together are about 5.6% self time.

## Optimization candidates, in rough order of payoff

1. **A cheaper path for sync-lowered host calls** (about 15% of samples are
   calls that do almost nothing).
   - Determine the current scope and its `CallContext` once per host call and
     pass them through lifting, borrow tracking and `validate_scope_exit`,
     rather than looking them up in each step. Do the same for
     `component_resource_tables_and_host_resource_data` and
     `options_memory`.
   - Make the deferred-host `set_thread` cheaper. A deferred host "thread"
     can't call `context.get`/`set`, so the context save/restore and
     `handle_thread_switch` work may be skippable on entry and exit.
   - Reusing the `CallContext` across calls is low priority; see below.
2. **Typed tables for the async runtime's own objects** instead of the generic
   `ResourceTable`, avoiding a `Box<dyn Any>` and child `BTreeSet` per
   transmit, handle or task. This affects `request.new`, `consume-body`, all
   the `future.*` built-ins and `task.return`: roughly 25–30% of samples touch
   it.
3. **Cheaper future/stream setup.** One `request.new` or `consume-body` creates
   several futures and streams, each with a `TransmitState`, two
   `TransmitHandle`s and handle-table entries. Allocating these together, or
   creating them lazily, would cut both resource-table and allocator time.
4. **Leaner export calls (host → guest).** Avoid boxing in
   `stage_call`/`StagedCall`, make `GuestTask::new` and `cleanup_thread`
   cheaper (the latter includes hashbrown lookups by `TableId`), and check
   whether the fiber set up in `make_fiber_unchecked` is reused across
   requests.
5. **Fewer large moves**, to reduce the 6% `memcpy`: box or pin large futures
   once, and shrink the types that `new_transmit` and `into_http_with_getter`
   move around.
6. **wasi-http itself, lower priority:** `client.send`'s channel and
   join-handle plumbing (5.9%), and URI rebuilding in
   `set-path-with-query`/`set-authority`.

## Reusing the `CallContext` (branch `abk/reuse-CallContext`)

The branch `abk/reuse-CallContext` keeps the deferred host call's
`CallContext` in `ConcurrentState` between calls, so it doesn't have to be
recreated each time. In this benchmark it showed no change or a slight
slowdown. That is expected, for two reasons.

1. **As written, it can't remove the allocation.** The allocation is the
   `lenders: Vec` inside `CallContext`. `validate_scope_exit`
   (`crates/wasmtime/src/runtime/vm/component/resources.rs`) consumes it with
   `for lender in mem::take(&mut cx.lenders)`, which frees the buffer at the
   end of every call. The retained `CallContext` therefore always has a
   zero-capacity `lenders`, and the next `resource_lift_borrow` allocates
   again. The branch only avoids building a small struct that never
   allocated, and it adds an enum `match` and some `mem::replace` moves per
   call.
2. **The possible gain is small.** Even if the allocation were removed, at
   most about 0.36% of samples (~60 ns per request) would be saved. That is
   within Criterion's run-to-run noise for this end-to-end benchmark.

**The branch can be fixed** by making `validate_scope_exit` keep the `Vec`'s
capacity instead of discarding it. Take the `Vec`, drain it while undoing the
lends, then put the now-empty `Vec` back into the scope's `cx.lenders`:

```rust
let mut lenders = mem::take(&mut self.current_scope()?.lenders);
for lender in lenders.drain(..) {
    self.table_for_index(&lender).resource_undo_lend(lender).unwrap();
}
self.current_scope()?.lenders = lenders;
```

Combined with the branch's retained `CallContext`, this removes the
malloc/free pair from the deferred host call path. The second
`current_scope()` lookup has a cost of its own, though, so the net effect may
be close to zero. It is best paired with the "look up the scope once per call"
change in item 1, and measured with the microbenchmark suggested below rather
than with the end-to-end proxy benchmark.

## Suggested next step

Before changing anything, add a microbenchmark to `benches/call.rs` (the
`component-async` group) for a sync-lowered import that takes a `borrow<R>`
and returns a small value. That isolates the ~350 ns per-call floor so each
change in items 1–2 can be measured directly.
