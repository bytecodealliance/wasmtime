# Steps 3 and 4: optimization benchmarks and profile

## Summary

- **Proxy latency:** `abk/opt-combined` cuts `p3_http_proxy` latency by
  **10.1%**, from 17.23 µs to 15.49 µs. `abk/opt3f-typed-table-boxed-over-48`
  does nearly as well on its own (−9.4%). `abk/opt2a-lazy-context-slots`
  gives −3.8%.
- **Per-API latency:** the typed table (opt3f) cuts 15–19% off
  `request.new`, `request.consume-body` and `response.consume-body`, and about
  4% off `client.send`. Adding opt2a on top gives no further gain on any
  individual API.
- **Where the remaining 15.3 µs goes:** host calls made by the guest still
  take **62%** of the time. The largest single costs are `client.send` (15%),
  `request.new` (9%), `request.consume-body` (6%) and `fields.copy-all` (5%).
- **Trivial calls are still pure overhead:** `get-method`, `set-method`,
  `set-scheme` and `fields.new` each still cost about 290–320 ns per call for
  almost no work.
- **Hidden costs:** `memcpy` (5% overall) and malloc/free (about 7%) are
  spread across everything. About 2 points of the `memcpy` come from moving
  large futures: the benchmark's future in `block_on`, and `client.send`'s
  host future.
- **Delayed cleanup found:** writing the API benchmarks turned up a delayed
  cleanup, not a true leak; see `steps_4_5_report.md`. When a guest drops a
  future's writable end while its readable end is open but unread, and later
  drops the readable end, about 3 host table entries stay behind per future
  until the guest task gets an event-loop turn for each one.

## Step 3: benchmark results

### Method

- **Branches:** each optimization branch's commits were cherry-picked onto
  `abk/p3-http-api-benches`. They were built on the original
  `abk/proxy-profiling`, and every cherry-pick applied cleanly.

  | Variant | Branch | Commits |
  |---|---|---|
  | base | `abk/p3-http-api-benches` | — |
  | opt2a | `abk/bench-opt2a` | `abk/opt2a-lazy-context-slots` (1 commit) |
  | opt3f | `abk/bench-opt3f` | `abk/opt3f-typed-table-boxed-over-48` (5 commits) |
  | opt-combined | `abk/bench-opt-combined` | `abk/opt-combined` (7 commits) |

- **Builds:** each variant's bench binary was built with
  `cargo bench -p wasmtime-wasi-http --bench p3_http_proxy --features test-programs-artifacts/release-artifacts --no-run`
  and copied aside. All variants use the same release-built guests.
- **Runs:**
  - The binaries were run interleaved (base, opt2a, opt3f, opt-combined,
    then repeat) for 3 rounds.
  - Each run was pinned with `taskset -c 2` and used `setarch -R`, with
    Criterion's 2 s warm-up and 6 s measurement.
  - CPU settings were as `~/bench-tune.sh` sets them: turbo off, and the
    `performance` governor and EPP on every core. They were already applied,
    so the script wasn't rerun.
- **Reported numbers:** each value is the median of the three runs' Criterion
  means. ± is half the min–max spread across those runs.
- **Noise:** as in `proposals_results.md`, code layout alone can move a build
  by about ±0.5%. So differences under about 1% between variants aren't
  meaningful.

### Results

| Benchmark | base | opt2a | opt3f | opt-combined |
|---|---|---|---|---|
| `p3_http_proxy` | 17.234 µs ±0.3% | 16.579 µs (**−3.8%**) ±0.1% | 15.611 µs (**−9.4%**) ±0.1% | 15.492 µs (**−10.1%**) ±0.2% |
| `p3-api/request.new` | 1.093 µs ±0.1% | 1.009 µs (−7.7%) ±0.7% | 0.903 µs (**−17.4%**) ±0.2% | 0.907 µs (−17.0%) ±0.1% |
| `p3-api/request.consume-body` | 0.815 µs ±0.1% | 0.696 µs (−14.6%) ±0.3% | 0.691 µs (**−15.3%**) ±0.1% | 0.698 µs (−14.5%) ±0.3% |
| `p3-api/response.consume-body` | 1.541 µs ±0.5% | 1.420 µs (−7.8%) ±0.1% | 1.244 µs (**−19.3%**) ±0.5% | 1.259 µs (−18.3%) ±1.1% |
| `p3-api/client.send` | 3.054 µs ±0.2% | 2.916 µs (**−4.5%**) ±0.2% | 2.929 µs (−4.1%) ±0.2% | 2.970 µs (−2.8%) ±0.3% |

### Analysis

- **opt3f (typed table) gives most of the gain.** It does best on the
  calls that create and destroy async runtime objects: `request.new`, and
  both `consume-body`s, which create a stream, a trailers future and their
  handles. That fits the change, which moves those objects out of the
  generic `ResourceTable` and its `Box<dyn Any>` entries.
- **opt2a overlaps opt3f almost completely.** On its own it helps every API
  (−4.5% to −14.6%). Stacked on opt3f it adds nothing to any individual API,
  and only 0.7 points to the proxy. `proposals_results.md` saw the same
  thing: opt2a saves generic table lookups, which opt3f makes cheap anyway.
- **`client.send`** improves least (−3% to −5%). Most of its cost is in
  wasi-http and tokio rather than in the tables (see step 4). opt-combined is
  1.3 points worse than opt3f here, slightly more than layout noise would
  explain. That's worth a recheck if `client.send` becomes a focus.
- **Compared with `proposals_results.md`:** the base is 17.23 µs here, against
  17.4 µs before, measured on a different core and without ASLR. opt-combined
  measures −10.1% here against −12% before. Its absolute time is about the
  same (15.49 µs against 15.3 µs), so the smaller percentage mostly comes from
  the faster base.

**Recommendation:** opt3f is the change worth landing. opt2a is a small
extra on the full proxy, and nothing on top of opt3f for individual calls.

## Step 4: profile of `p3_http_proxy` with opt-combined

### Method

- **Build:** `abk/bench-opt-combined`, built with
  `cargo bench --profile profiling` (line tables) and
  `RUSTFLAGS="-C force-frame-pointers=yes"`, in a separate target dir. It
  benchmarks at 15.32 µs, the same as the plain build, so the frame pointers
  don't distort the timing.
- **Recording:** `perf record -F 10000 --call-graph fp -- taskset -c 2 setarch -R <bench> --bench --profile-time 20 wasi-http/p3_http_proxy`,
  with the CPU tuned as in step 3.
- **Samples:** 205,652 in total. 6,151 startup samples (compiling the
  component) are excluded, leaving 199,500.
- **Unwinding:** frame pointers let `perf` unwind through host code, Wasmtime
  trampolines and guest JIT code. Guest functions are named from the
  benchmark's perf map, so each sample can be attributed to the guest import
  (via its wit-bindgen wrapper) or canonical built-in (via its trampoline)
  that led to it.
- **Limits:** stacks were resolved without inlined frames, so "layer" below
  means the layer of the leaf (non-inlined) function. That's approximate, and
  host functions with big inlined bodies are attributed to their outer
  function.
- **Scale:** at about 15.3 µs per request, **1% ≈ 153 ns per request**.

### Where the time goes

| Region | Share | ≈ per request |
|---|---|---|
| Inside host calls made by the guest | **62.2%** | 9.5 µs |
| Host work with no guest frame on the stack: harness, event loop, request/response conversion | 16.8% | 2.6 µs |
| Starting, resuming and finishing guest calls and fibers | 6.9% | 1.1 µs |
| Guest code: allocator ~2%, URL parsing ~2.5%, wit-bindgen runtime ~1% | 13.2% | 2.0 µs |
| Trampolines | 0.7% | 0.1 µs |

The 16.8% "harness" region is mostly per-request work that any embedder
would do, inside the benchmark's timed loop:
- building the `http::Request` and running `Request::from_http`;
- `Response::into_http` and collecting the body;
- the `try_join!`/`poll_fn` driving the request;
- tokio's `block_on`.

It's split across two buckets in the raw analysis because inlining hides
which function each sample was in.

### Host calls from the guest, by call

| Call | Share | ≈ per call | Biggest layers |
|---|---|---|---|
| `client.send` | **14.6%** | 2.2 µs | wasi-http impl 4.9, tasks/threads/scopes 3.2, alloc/memcpy 2.5, glue 1.9 |
| `request.new` | **9.0%** | 1.4 µs | futures/streams 2.4, glue 2.0, tables 1.8, alloc 0.9 |
| `request.consume-body` | **6.4%** | 1.0 µs | glue 1.4, futures/streams 1.4, tables 1.3, alloc 0.8 |
| `fields.copy-all` | 5.1% | 780 ns | glue 2.9 (lowering a list of strings), alloc 0.7 |
| `request.set-path-with-query` | 3.5% | 540 ns | glue 1.3, wasi-http 0.9 (URI parsing) |
| built-in `future.write` | 3.2% | 490 ns | futures/streams 1.8 |
| `request.get-headers` | 2.9% | 440 ns | glue 1.4, tables 0.7 |
| built-in `task.return` | 2.5% | 380 ns | tokio 0.8, tasks 0.6, glue 0.6 |
| `request.set-authority` | 2.3% | 350 ns | glue 1.1 |
| `fields.new` | 2.1% | 320 ns | glue 1.0, tables 0.5 |
| `request.set-method` / `get-method` / `set-scheme` | 1.8–1.9% each | ~290 ns each | glue ~1.0 each; almost no actual work |
| built-in `future.drop-readable` + `future.drop-writable` | 2.8% | 430 ns | futures/streams 1.3, alloc 0.6 |
| built-in `future.new` | 0.8% | 120 ns | tables 0.4 |
| resource drops | 1.1% | 170 ns | tables 0.7 |

These assume each call happens once per request, which matches the guest
code. Calls weren't counted directly.

### Host calls from the guest, by layer

| Layer (leaf function) | Share |
|---|---|
| Lift/lower and host-call glue (`StaticHostFn`, `LowerContext`, `options_memory_mut`, typed lowering) | **16.8%** |
| Resource and handle tables | 8.5% |
| wasi-http implementation (including the `client.send` future body) | 8.4% |
| Tasks, threads and scopes (`host_task_create`/`delete`, `set_thread`, `current_scope`) | 7.5% |
| Futures and streams (transmit state) | 7.1% |
| malloc/free/memcpy | 6.4% |
| Other | 6.1% |
| tokio/futures runtime | 1.1% |

Compared with `proxy_profile_report.md`, measured before these optimizations,
the tables are down from 15.7% to 8.5% of samples. The fixed cost of a
host-call (glue, scopes, tasks) is now the largest layer.

### Hottest functions (self time, all regions)

| Self | Function | Notes |
|---|---|---|
| 5.1% | `__memmove_avx_unaligned_erms` | 1.1% from `block_on` (moving the benchmark's future), 1.0% from `client.send`'s `Instrumented` host future, then many small callers |
| 2.5% + 1.8% + 1.2% + … | `cfree`, `malloc`, `__rdl_alloc`, … | About 7% total. Biggest callers: `host_drop_reader`, `client.send`, the `StaticHostFn` glue, `consume_body` |
| 2.0% | `StoreOpaque::component_resource_tables_and_host_resource_data` | Called on every host call that touches a resource |
| 1.4% | `Request::into_http_with_getter` | `client.send` converting the guest request into an `http::Request` |
| 1.4% | `LowerContext::validate_scope_exit` | Every host call |
| 1.3% | `HandleTable::insert` | |
| 1.3% | `Instance::options_memory_mut` | 0.7% of it is inside `fields.copy-all`, apparently looked up again for each lowered value |
| 1.2% | `StoreOpaque::current_scope` | Every host call |
| 1.1% | `StoreOpaque::host_task_create` | Every host call |
| 0.8% | `ConcurrentState::delete_transmit` | |

### Areas that contribute most to latency

1. **Fixed cost of each host call (largest overall).** There are 15 sync host
   calls per request. Glue, scope, task and table work for each costs at
   least about 290 ns, even when the call does nothing. That's roughly
   4–5 µs per request in total. The main parts are:
   - `host_task_create`/`host_task_delete`/`set_thread`;
   - `current_scope`;
   - `validate_scope_exit`;
   - `component_resource_tables_and_host_resource_data`;
   - `options_memory_mut`;
   - the `StaticHostFn` entry and lowering.

   A fast path for sync, non-reentrant host calls would help every API.
2. **`client.send`, about 2.2 µs.**
   - About 1/3 is wasi-http work: `into_http_with_getter`, building the
     `Uri` and `HeaderMap`, the oneshot channels and `AbortOnDropJoinHandle`,
     and `send_dummy_io`.
   - About 1/4 is task and thread bookkeeping for an async-lowered call.
   - About 1 point is `memcpy` from moving its large `Instrumented` future
     around.
3. **Creating and destroying futures and streams (body plumbing).** Per
   request this takes about 3 µs: `request.new` plus `consume-body`, plus
   the `future.new`/`write`/`drop-*` built-ins and their transmit state.
   opt3f made the table side cheaper. What remains is mostly `new_transmit`,
   `delete_transmit`, `host_reader_take`/`host_drop_reader`, `set_consumer`,
   and the allocations behind them.
4. **Request/response conversion and event-loop work outside guest calls,
   about 2.6 µs.** This covers `Request::from_http`, `Response::into_http`,
   body collection, tokio, and moving the benchmark's future in `block_on`.
   Part of this is the benchmark's own harness, but `from_http` and
   `into_http` are real costs for every embedder.
5. **Entering and resuming guest calls and fibers, about 1.1 µs.** This is
   `resume_fiber`, `suspend`, `cleanup_thread` and `stage_call`.
6. **Guest code, about 2 µs.** Mostly the guest's allocator (dlmalloc) and
   URL parsing in `p3_http_proxy` itself. This is specific to this guest and
   largely out of scope.

## Leak found while building the API benchmarks

**Correction:** this section originally described a permanent leak. Further
investigation in `steps_4_5_report.md` (step 5) showed that the entries are
freed, one future per event-loop turn, once the guest task gets those turns.
They pile up because nothing drives the event loop for that long after the
export returns, or while the guest is busy. See that report for the
reproduction on `main`, the mechanism and possible fixes.

## Reproducing

- **Benchmark branches:** `abk/bench-opt2a`, `abk/bench-opt3f`,
  `abk/bench-opt-combined`, on top of `abk/p3-http-api-benches`. None of them
  are pushed.
- **Run one variant:**
  `cargo bench -p wasmtime-wasi-http --bench p3_http_proxy --features test-programs-artifacts/release-artifacts`
- **Profile data:** `perf.data` and the bench binary it was recorded from
  are in the session scratchpad:
  `/tmp/claude-1000/-home-adam-wasmtime/4d0010e7-cb9b-47aa-aea4-e8c120526c0c/scratchpad/perf.data`
  and `/tmp/claude-1000/-home-adam-wasmtime/4d0010e7-cb9b-47aa-aea4-e8c120526c0c/scratchpad/bins/opt-combined-prof`.
  The analysis scripts are `analyze.py` and `callers.py` in the same
  directory.
- **Not tested:** I didn't rerun the test suites on the `abk/bench-*`
  branches. Their optimization commits are unchanged from the original
  branches, which passed the suites listed in `proposals_results.md`.
