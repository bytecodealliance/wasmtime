# Steps 4 and 5: profile on current `main`, and the future-writer "leak"

## Summary

- **The stack is rebased onto current `main`** (`a956f4d5d3`, 65 commits newer
  than before). `abk/opt3f-typed-table` puts opt3f on top of the benchmarks.
  It needed a small port for `main`'s #14706 (`len()` and `ensure_capacity` on
  the concurrent table) and passes the same suites as before, plus `main`'s
  new `async_table_capacity` tests.
- **opt3f still helps on current `main`; the apparent regression was a code
  layout effect.** With the default 16 codegen units, opt3f measured −4.4% on
  the proxy and +12% on `request.consume-body`. Bisecting `main` pinned the
  change to #14687, a backtrace fix that touches no component code. It moves
  how rustc partitions `wasmtime` into codegen units, and so what LLVM
  inlines. Built with `codegen-units=1`, opt3f gives −9.0% on the proxy and
  −3.8% on `request.consume-body` on current `main`, and the numbers are the
  same before and after #14687. See "Bisecting the opt3f regression" below.
- **Profile, with opt3f, at 16.35 µs per request:**
  - **Host calls made by the guest:** 65.5%.
  - **Guest code:** 13.9%.
  - **Host work outside guest calls:** 19.8%. That's the harness, the event
    loop, and starting and resuming guest calls.
  - **Largest costs:** the fixed overhead of each host call, `client.send`,
    and the future/stream plumbing for bodies.
- **The "leak" is present on `main`, but it isn't a permanent leak.** When a
  wit-bindgen guest drops a future's writer before its reader, about 3 table
  entries per future stay behind after the export returns. Each future is
  cleaned up only after the guest task gets an event-loop turn of its own,
  and `run_concurrent` stops driving the loop once the export has returned.
  The same sequence in raw WAT doesn't leave anything behind. My earlier
  report called this a real leak; that was wrong, and this report corrects it.

## Branches

| Branch | Base | Contents |
|---|---|---|
| `abk/release-test-artifacts` | `main` | `test-programs: Release mode feature` (your amended version) |
| `abk/p3-http-proxy-benchmark` | ↑ | `wasi-http: Add a benchmark of a p3 wasi:http proxy` (your amended version) |
| `abk/p3-http-api-benches` | ↑ | `Add benchmarks for individual wasi:http p3 APIs` |
| `abk/opt3f-typed-table` | ↑ | The 5 commits of `abk/opt3f-typed-table-boxed-over-48`, ported to current `main` |
| `abk/future-writer-leak` | `main` | Reproduction tests for step 5, in the worktree `/home/adam/wasmtime-leak` |

None of them are pushed.

**Rebase notes:**
- **Your amended commits are kept:** your amends to the bottom two commits
  (shorter comments, reworded messages, `proxy_profile.sh` and
  `www/index.html` dropped) are kept as you wrote them.
- **API-benches conflicts:** rebasing the API-benches commit conflicted in
  two places:
  - The bench's doc comment, now one line in your shorter style.
  - `tests/all/p3/mod.rs`, where `main` added
    `p3_http_proxy_store_drop_cancels_io` next to the new test. Both tests
    are kept.
- **Corrected wording:** I amended the API-benches commit again to fix how
  its comment and message describe the leak.

**Porting opt3f to current `main`:** #14706 ("Track live resources and
preflight component async table allocations") added several things that
assume the concurrent table is a `ResourceTable`:
- `ResourceTable::len()` and `ensure_capacity`;
- a `new_transmit` preflight;
- `Store::concurrent_resource_table()` docs and tests that call
  `.len()`/`.set_max_capacity()`;
- `wast` and CLI hooks that do the same.

The port, folded into opt3f's first commit:
- **`ConcurrentTable`:** gains O(1) `len()`/`is_empty()` via `live` and
  `free_count` counters, plus `ensure_capacity`, with the same semantics as
  `ResourceTable`.
- **`ResourceTable`:** `ensure_capacity` and `free_count` move off it, since
  only the concurrency code used them. `ResourceTable` keeps `main`'s `len()`.
  Its batch-capacity unit test moves to `ConcurrentTable`.
- **Docs:** `Store::concurrent_resource_table` docs now refer to
  `ConcurrentTable`.

**Verification on `abk/opt3f-typed-table`:**
- `cargo test -p wasmtime --lib -- concurrent resource_table` (22)
- `cargo test --test all -- component_model async_table_capacity` (283)
- `cargo test --test wast -- component-model` (408)
- `cargo test -p component-async-tests` (78)
- `cargo test -p wasmtime-wasi-http --test all p3` (34)
- A workspace `cargo check --all-targets` is clean. `wasmtime-c-api` was left
  out because its build script needs `cmake`, which isn't installed here.

## Benchmarks on current `main`

Same method as `steps_3_4_report.md`:
- interleaved runs, 3 rounds;
- `taskset -c 2`, `setarch -R`;
- 2 s warm-up and 6 s measurement;
- turbo off, `performance` governor and EPP;
- the median of the per-run means.

`opt3f-unboxed` is opt3f's first commit only: the typed table, without the
later boxing commits.

| Benchmark | base | opt3f-unboxed | opt3f |
|---|---|---|---|
| `p3_http_proxy` | 17.094 µs ±0.3% | 16.523 µs (−3.3%) ±0.1% | 16.347 µs (**−4.4%**) ±0.3% |
| `p3-api/request.new` | 1.059 µs ±0.1% | 1.120 µs (+5.7%) ±0.2% | 1.049 µs (−1.0%) ±0.1% |
| `p3-api/request.consume-body` | 0.727 µs ±0.3% | 0.830 µs (+14.2%) ±0.2% | 0.816 µs (**+12.2%**) ±0.2% |
| `p3-api/response.consume-body` | 1.501 µs ±0.4% | 1.563 µs (+4.1%) ±0.7% | 1.442 µs (−3.9%) ±0.8% |
| `p3-api/client.send` | 2.731 µs ±0.3% | 2.824 µs (+3.4%) ±0.3% | 2.795 µs (+2.3%) ±0.3% |

An earlier interleaved run of base against opt3f agreed to within 0.2 points
on every row.

**Compared with the old base:** `main` itself got faster on the APIs:
- `request.consume-body`: 0.815 → 0.727 µs.
- `client.send`: 3.054 → 2.731 µs.
- `response.consume-body`: 1.541 → 1.501 µs.

Meanwhile opt3f got slower on `consume-body` (0.691 → 0.816 µs). So most of
what opt3f used to save on the API benchmarks is gone, and `consume-body`
regresses.

**Update:** the bisect below shows this table measures codegen-unit luck as
much as opt3f. Use the `codegen-units=1` numbers instead.

### Bisecting the opt3f regression

**Method:**
- At each test point on `main`'s first-parent history, cherry-pick the three
  benchmark commits, then opt3f, then build the bench for base and opt3f.
- Commits before #14732 and #14706 use the earlier opt3f commits. The ones
  after #14732 needed a trivial `store.rs` conflict resolution.
- Benchmarks as above, filtered to the proxy, both `consume-body` APIs and
  `client.send`.
- The scripts and raw results are in `target/bisect/` (`build.sh`,
  `build_cgu1.sh`, `run.sh`, `runs/`, `log*.txt`). Point *N* is the *N*th
  first-parent commit after the old base `9a79d52a2e`.

**`request.consume-body` with the default 16 codegen units:**

| Point | Commit | base | opt3f | Change |
|---|---|---|---|---|
| p00 | old base `9a79d52a2e` | 0.820 µs | 0.692 µs | −15.5% |
| p10 | `9873ab2bef` #14709 | 0.829 µs | 0.696 µs | −16.0% |
| p11 | `2b85e305af` #14690 | 0.821 µs | 0.691 µs | −15.9% |
| p15 | `ac3e89b721` #14711 | 0.818 µs | 0.693 µs | −15.3% |
| **p16** | **`c76569e2bd` #14687** | 0.820 µs | **0.806 µs** | **−1.8%** |
| p17 | `0072becdb3` #14628 | 0.816 µs | 0.801 µs | −1.8% |
| p21 | `4c8d4f6895` #14737 | 0.818 µs | 0.795 µs | −2.8% |
| p24 | `7ce0d50cde` #14686 | 0.818 µs | 0.795 µs | −2.9% |
| p25 | `28c6adad20` #14732 | 0.844 µs | 0.819 µs | −3.0% |
| p53 | `d4fb9b0842` #14757 | 0.836 µs | 0.812 µs | −2.8% |
| p54 | `a559c7fe6c` #14706 | 0.829 µs | 0.821 µs | −0.9% |
| p57 | `6876e31795` #14755 | 0.844 µs | **0.712 µs** | −15.6% |
| p65 | `main` `a956f4d5d3` | **0.735 µs** | 0.817 µs | +11.2% |

Base and opt3f each jump between a fast and a slow value of about 0.70 and
0.82 µs, at commits that don't touch this code:
- **#14687:** a stack-overflow backtrace fix for s390x and Pulley. It only
  changes `traphandlers.rs`, `signals.rs`, `interpreter.rs` and similar files.
  It makes opt3f slow.
- **#14755:** fixes OOM handling for host resources. It makes opt3f fast
  again.
- **Somewhere in p57–p65:** base becomes fast while opt3f becomes slow again.

**Why:**
- **Inlining:** the `wasmtime` crate is built as 16 codegen units, and
  non-generic functions only get inlined within their own unit. Adding or
  removing code anywhere in the crate changes how modules are merged into
  units, and so what gets inlined into the hot path.
- **Binary differences:** between p15 and p16 the opt3f binary differs in 39
  component functions (base in 5), all with unchanged source.
  - `ResourceTables::resource_rep` and `TypedResource::rep` stop being
    inlined into `Instance::resource_rep32`.
  - `HandleTable::remove_resource` gains four callee-saved register
    pushes/pops.
  - Many `Instance::*` intrinsics grow by about 20 bytes.
- **Counters (`perf stat`):** about 2% more instructions and about 3% lower
  IPC per iteration. That's roughly the 0.11 µs the timed loop lost.

**Confirmation with `CARGO_PROFILE_BENCH_CODEGEN_UNITS=1`:**

| Point | `p3_http_proxy` | `request.consume-body` | `response.consume-body` | `client.send` |
|---|---|---|---|---|
| p15 (before #14687) | 16.346 → 15.219 µs (−6.9%) | 0.722 → 0.695 µs (−3.7%) | 1.427 → 1.219 µs (−14.6%) | −0.6% |
| p16 (#14687) | 16.235 → 15.090 µs (−7.1%) | 0.719 → 0.695 µs (−3.3%) | 1.410 → 1.210 µs (−14.2%) | −1.0% |
| p65 (`main`) | 16.244 → 14.774 µs (**−9.0%**) | 0.737 → 0.709 µs (−3.8%) | 1.477 → 1.282 µs (−13.2%) | −0.3% |

With one codegen unit, #14687 makes no difference and opt3f is consistently
faster.

**Conclusions:**
- **opt3f didn't stop helping:** it's worth −7 to −9% on the proxy. The
  −15% on `request.consume-body` in `steps_3_4_report.md` was partly the old
  base being on the slow side of the layout lottery. With one codegen unit,
  opt3f is worth only about −3.5% there.
- **Measure with one codegen unit:** default-profile results on these
  benchmarks can swing by ±12% for reasons unrelated to the change being
  measured. Comparisons should use `codegen-units=1`, or check the result
  against it.
- **A possible follow-up:** making the hot handle-table and resource helpers
  `#[inline]`, so the results don't depend on partitioning, might lock in the
  fast variant. That is a separate change.

### What 16 codegen units fail to inline (p16 + opt3f)

**Setup:**
- **Branch:** `abk/p16-opt3f` is `c76569e2bd` (#14687) plus the three benchmark
  commits and opt3f's 5 commits.
- **Builds:** two, both with `RUSTFLAGS="-C remark=inline -C debuginfo=1"`, one
  with the default 16 codegen units and one with 1. They're in separate
  target dirs under `target/remarks/`.
- **Remarks:** 670K unique decisions for the 16-unit build and 736K for the
  single-unit build. They're matched by demangled name, since the crate
  disambiguators differ between the builds.
- **Profiles:** `perf --call-graph lbr`, matched against the remarks the same
  way.

| Benchmark | 16 units | 1 unit |
|---|---|---|
| `p3_http_proxy` | 16.117 µs | 15.108 µs (−6.3%) |
| `request.new` | 0.987 µs | 0.907 µs (−8.1%) |
| `request.consume-body` | 0.786 µs | 0.698 µs (−11.2%) |
| `response.consume-body` | 1.324 µs | 1.210 µs (−8.6%) |
| `client.send` | 3.068 µs | 2.838 µs (−7.5%) |

The table below lists functions with self time in the 16-unit
`request.consume-body` profile and none in the single-unit one, meaning they're
inlined there. Each row gives the remark that explains the difference. Self
time includes any `memmove` the function calls directly. In
`request.consume-body` these rows add up to about 12% of samples. In the proxy
they add up to about 16%.

| Self time (16 units) | Function, and where it's called | 16 units | 1 unit |
|---|---|---|---|
| 1.83% | `ConcurrentTable::delete::<TransmitState>`, called from `ConcurrentState::delete_transmit` | definition unavailable | inlined |
| 1.34% | `StoreContextMut::set_consumer::<pipe::Consumer<…Body::consume…>>`, called from `FutureReader::pipe` | definition unavailable | inlined (cost −11520, single caller) |
| 0.3–1.0% each | `StaticHostFn::<add_to_linker_instance::{closure#N}>::entrypoint`, called from its `HostFn` wrapper (closures #4, #14, #25 and #37 here; about 10 in the proxy) | definition unavailable | inlined (cost −11620, single caller) |
| 0.90% | `WasiHttp::consume_body`, called from `add_to_linker_instance::{closure#25}` | definition unavailable | inlined |
| 0.90% | `ConcurrentTable::push::<TransmitState>`, called from `ConcurrentState::new_transmit` | definition unavailable | inlined |
| 0.72% | `StoreContextMut::new_transmit::<FutureReader::new::Producer<…>>` | — | inlined |
| 0.52% | `host::types::new_request` | — | inlined |
| 0.39% | `Waitable::set_event`, called from `ConcurrentState::update_event` | too costly | inlined |
| 0.31% | `TypedResource::rep`, called from `HandleTable::remove_resource`, `resource_lend` and `resource_rep` | definition unavailable | inlined (cost 175, threshold 250) |

The proxy profile adds:
- `Request::into_http_with_getter`, called from `handler::send` (2.16%): definition
  unavailable with 16 units, inlined with 1.
- The bench harness's `proxy_one_request` `PollFn` (1.91%).
- `http::request::Builder::{header, uri}`.
- Fiber `resume`/`suspend`.

**Reading the remarks:**
- **"Definition unavailable":** this is the codegen-unit split. The callee was
  placed in another unit, and ThinLTO doesn't import a function that large.
- **Single-caller bonus:** most of the single-unit wins carry LLVM's large
  negative cost for this, so they would always be inlined if they were in the
  same unit.
- **Copy cost:** `ConcurrentTable::{push, delete}::<TransmitState>` are what make
  opt3f sensitive here. They're opt3f's own generic table methods, and since
  neither is `#[inline]`, each `TransmitState` is copied by value through an
  out-of-line call. That's why `memmove` is 4.1% of `consume-body` samples
  with 16 units against 1.9% with 1.
- **Source locations:**
  - `ConcurrentTable::push`/`delete` are in `concurrent/table.rs`.
  - `TypedResource::rep` is in `vm/component/resources.rs`, called from
    `handle_table.rs`.
  - `Waitable::set_event` is in `concurrent.rs`.
- **Bench-crate entries:** the `StaticHostFn`, `set_consumer`, `new_transmit` and
  wasi-http entries are generic instantiations compiled into the bench crate.
  They affect base and opt3f alike.

**Candidate fixes to try:**
- `#[inline]` on `ConcurrentTable::{push, delete, get, get_mut}` and on
  `TypedResource::rep`.
- Then re-measure with 16 codegen units, to see whether it recovers the
  single-unit numbers for opt3f.

### The same analysis on p16 without opt3f

**Setup:** `abk/p16-base` is `abk/p16-opt3f` without opt3f's 5 commits. It was
built and analyzed the same way, in `target/remarks-base/`.

| Benchmark | base, 16 units | base, 1 unit | opt3f, 16 units | opt3f, 1 unit |
|---|---|---|---|---|
| `p3_http_proxy` | 17.098 µs | 16.203 µs (−5.2%) | 16.117 µs | 15.108 µs |
| `request.new` | 1.086 µs | 1.027 µs (−5.4%) | 0.987 µs | 0.907 µs |
| `request.consume-body` | 0.822 µs | 0.715 µs (−13.0%) | 0.786 µs | 0.698 µs |
| `response.consume-body` | 1.529 µs | 1.421 µs (−7.1%) | 1.324 µs | 1.210 µs |
| `client.send` | 3.114 µs | 2.840 µs (−8.8%) | 3.068 µs | 2.838 µs |

**Base is just as sensitive to codegen units.** In base's 16-unit build, the
functions left out-of-line account for 11.2% of `request.consume-body` samples
(opt3f: 12.1%) and 14.8% of the proxy samples (opt3f: 16.2%).

**Most entries are the same in both builds:**
- the bench-crate instantiations: the `StaticHostFn` entry points,
  `set_consumer::<…>`, `new_transmit::<…>`, `WasiHttp::consume_body`,
  `new_request`, and `Request::into_http_with_getter`;
- `TypedResource::rep`;
- `Waitable::set_event`.

Their costs are similar in both.

**What differs is the table:**

| Function (16 units) | base | opt3f |
|---|---|---|
| Table delete of `TransmitState`, called from `delete_transmit` (`ResourceTable::delete_maybe_debug` in base, `ConcurrentTable::delete` in opt3f) | 2.19% | 1.83% |
| `ConcurrentTable::push::<TransmitState>`, called from `new_transmit` | — (inlined) | 0.90% |
| `ResourceTable` helpers: `BTreeMap` drop, `drop_glue::<Entry>`, `get_mut::<TransmitHandle>`, `ConcurrentState::get_mut::<TransmitState>` | 1.61% | — (gone) |
| `memmove`, all callers | 2.98% (1 unit: 2.55%) | 4.11% (1 unit: 1.92%) |

**Conclusions:**
- **Both builds:** with 16 units, deleting a `TransmitState` happens out of line,
  copying the state out of its box.
- **opt3f's tradeoff:** it removes the `ResourceTable` overhead, the `BTreeMap`
  of children and the `dyn Any` entries. But it adds one more out-of-line,
  by-value generic, `push::<TransmitState>`.
  - When it's inlined (1 unit), opt3f copies the least: `memmove` is at 1.92%.
  - When it isn't, it copies the most: `memmove` is at 4.11%.
  - So opt3f's gain over base varies with the build:

    | Benchmark | 16 units | 1 unit |
    |---|---|---|
    | `request.consume-body` | −4.4% | −2.4% |
    | Proxy | −5.7% | −6.8% |

- **Fixes:**
  - The bench-crate entries account for most of the out-of-line cost in both
    builds. These would need `#[inline]` on the generic host-call plumbing in
    `wasmtime`, such as `StaticHostFn::entrypoint` and `set_consumer`, rather
    than on the table.
  - The `#[inline]` candidates from the opt3f analysis remain:
    `ConcurrentTable::{push, delete}`, or `ResourceTable::delete_maybe_debug`
    for base, plus `TypedResource::rep`.

## Step 4: profile of `p3_http_proxy` with opt3f

### Method

- **Build:** `abk/opt3f-typed-table`, built with `--profile profiling` and
  `-C force-frame-pointers=yes` in a separate target dir.
- **Recording:** `perf record -F 10000 --call-graph fp -- taskset -c 2 setarch -R <bench> --bench --profile-time 20 wasi-http/p3_http_proxy`,
  with the CPU tuned as above.
- **Samples:** 200,413 after excluding startup.
- **Analysis:** same as in `steps_3_4_report.md`. Samples are attributed
  through the guest's JIT frames to the WASI call or canonical built-in that
  led to them. Layers come from the leaf, non-inlined function, so they're
  approximate.
- **Scale:** at 16.35 µs per request, **1% ≈ 164 ns**.

### Where the time goes

| Region | Share | ≈ per request |
|---|---|---|
| Inside host calls made by the guest | **65.5%** | 10.7 µs |
| Host work with no guest frame: harness, event loop, request/response conversion | 12.9% | 2.1 µs |
| Starting, resuming and finishing guest calls and fibers | 6.9% | 1.1 µs |
| Guest code: allocator ~2.5%, URL parsing ~2%, wit-bindgen runtime ~2% | 13.9% | 2.3 µs |
| Trampolines | 0.6% | 0.1 µs |

### Host calls from the guest, by call

| Call | Share | ≈ per call | Biggest layers |
|---|---|---|---|
| `client.send` | **12.4%** | 2.0 µs | tasks/threads/scopes 3.3, wasi-http 3.3, glue 2.1, alloc 1.9 |
| `request.new` | **8.9%** | 1.5 µs | glue 2.4, futures/streams 2.3, tables 1.8 |
| `request.consume-body` | **8.8%** | 1.4 µs | glue 2.3, futures/streams 2.2, tables 1.6 |
| `fields.copy-all` | 6.0% | 980 ns | glue 3.3, alloc 0.9 |
| built-in `future.write` | 3.7% | 610 ns | futures/streams 2.0 |
| `request.set-path-with-query` | 3.6% | 590 ns | glue 1.3, wasi-http 0.9 |
| `request.get-headers` | 3.4% | 560 ns | glue 1.6, tables 0.8 |
| `request.set-authority` | 2.7% | 440 ns | glue 1.2 |
| built-in `task.return` | 2.6% | 430 ns | tokio 0.8, glue 0.6, tasks 0.6 |
| `request.get-method` / `set-method` / `set-scheme` / `fields.new` | 1.8–2.4% each | 300–390 ns each | glue ≈1 each; almost no actual work |
| built-in `future.drop-readable` + `future.drop-writable` + `future.new` | 3.8% | 620 ns | futures/streams 1.7 |
| resource drops | 1.2% | 200 ns | tables 0.8 |

These assume each call happens once per request, which matches the guest
code.

By layer, the host calls split as follows:

| Layer | Share |
|---|---|
| Lift/lower and host-call glue | **19.5%** |
| Tables | 9.1% |
| Tasks, threads and scopes | 8.4% |
| Futures and streams | 8.3% |
| wasi-http | 7.1% |
| malloc/memcpy | 5.9% |
| Other | 6.1% |
| tokio | 1.0% |

### Hottest functions (self time)

| Self | Function |
|---|---|
| 4.1% | `memmove`. Top callers: `client.send`'s `Instrumented` host future (0.8%), `block_on` moving the benchmark's future (0.8%), `delete_transmit` (0.2%) |
| ≈7% total | malloc/free: `host_drop_reader`, the `StaticHostFn` glue, `client.send`, `consume_body`, `resource_lift_borrow`, … |
| 2.1% | `StoreOpaque::component_resource_tables_and_host_resource_data` |
| 1.4% | `LowerContext::validate_scope_exit` |
| 1.4% | `Request::into_http_with_getter` (`client.send`) |
| 1.4% | `Instance::options_memory_mut` |
| 1.3% | `HandleTable::insert` |
| 1.2% + 1.1% + 0.5% | `host_task_create`, `set_thread::<CurrentThread>`, `set_thread::<QualifiedThreadId>` |
| 1.1% | `StoreOpaque::current_scope` |
| 1.1% | guest: `wit_bindgen::rt::async_support::spawn::Tasks::new`, once per `handle` call |
| 0.8% + 0.8% + 0.7% | `resource_lower_own`, `resource_lift_own`, `resource_lift_borrow` |

### Areas to investigate

1. **Fixed cost of a host call: about 300–400 ns even for a no-op, roughly
   5 µs per request.**
   - The proxy makes 15 sync host calls per request. The trivial ones
     (`get-method`, `set-method`, `set-scheme`, `fields.new`) do almost no
     work, so their cost is all overhead:
     - `host_task_create`/`host_task_delete` with two `set_thread` switches;
     - `current_scope` and `validate_scope_exit`;
     - `component_resource_tables_and_host_resource_data`;
     - `options_memory_mut`;
     - the `StaticHostFn` entry, lifting and lowering.
   - A fast path for sync-lowered host functions that can't block or
     re-enter would help every call. It would skip materializing a host task
     and switching threads, and look the resource tables and memory up once
     per call instead of once per operand.
2. **Lowering lists of strings in `fields.copy-all` (6%, about 1 µs).**
   - More than half is glue. `options_memory_mut` alone is noticeable here,
     apparently looked up again for each lowered value, alongside
     `LowerContext::realloc`, which calls back into the guest for each
     string.
   - Looking memory up once per call, and allocating all the strings with a
     single guest allocation, should help any guest that reads headers.
3. **Body plumbing: futures and streams, about 3.5 µs per request.**
   - This covers `request.new`, `request.consume-body`, and
     `future.new`/`write`/`drop-*`.
   - Each future or stream costs 3 table entries and transmit state, plus
     host-side allocations: `host_reader_take`, `host_drop_reader`,
     `set_consumer`, the `Box`ed producers and consumers.
   - Bodies a proxy passes through untouched could avoid most of this. For
     example, create the transmit state lazily on first read or write, or
     hand a host body straight to an outgoing request without going through
     a guest stream.
4. **`client.send`, about 2 µs.**
   - About a quarter is async-lowered task bookkeeping.
   - About a quarter is wasi-http: `into_http_with_getter` rebuilds a `Uri`
     and converts `FieldMap`→`HeaderMap`; `send_dummy_io`, oneshot channels
     and `AbortOnDropJoinHandle` also cost.
   - About 0.8 points is `memcpy` of its large `Instrumented` future, which
     could be boxed once or slimmed down.
5. **Request and response conversion for the embedder (`from_http`,
   `into_http`, and body collection) and moving large futures in the event
   loop: about 2 µs.**
   - This is partly the benchmark's own harness, but every embedder pays for
     `from_http` and `into_http`.
6. **Codegen sensitivity of the hot path.** opt3f's effect didn't really
   shrink; see "Bisecting the opt3f regression". The hot handle-table and
   resource helpers swing by ±12% depending on codegen-unit partitioning.
   Marking them `#[inline]` may make the fast layout reliable, independent of
   unrelated changes elsewhere in the crate.
7. **Guest-side costs, specific to this guest:**
   - wit-bindgen allocates a `spawn::Tasks` per export call (1.1%);
   - dlmalloc (2%);
   - URL parsing (about 2%).

   These are mostly out of scope for host work, but the first could be a
   cheap wit-bindgen improvement.

## Step 5: the future-writer resource "leak"

### Symptom

- **When it happened:** the first version of the `p3_http_api_bench` command
  guest ran thousands of `request.new` or `request.consume-body` calls in one
  `run` call.
- **Error:** the host's concurrent table filled up to its 1,000,000-entry
  limit and the guest trapped with `resource table has no free keys`.
- **Trigger:** setup code that created futures with
  `wit_future::new(...)` and dropped the writer straight away, keeping only
  the reader. That's the same idiom `p3_http_proxy` uses for
  `consume-body`'s completion future.

### Minimal reproduction

The commit on `abk/future-writer-leak` adds a test program and a test:
- **Guest:** `p3_future_writer_drop` creates a
  `future<result<_, error-code>>` and drops both ends without writing, `iters`
  times. Either the writer or the reader goes first.
- **Test:** `p3_future_writer_drop` in `crates/wasi/tests/all/p3/mod.rs` runs
  it and checks `Store::concurrent_state_table_size()` once `run` has
  returned.

| Order | Entries left after `run` (1,000 futures) | Extra event-loop turns to drain |
|---|---|---|
| reader first | 0 | 0 |
| writer first | **3,004** (10 futures: 34; 100: 304) | **exactly 1 per future**, after which 0 are left |

**The behavior is present on `main`.** The test fails on `main`, so it's a
reproduction, not a regression test.

The same commit adds two raw-WAT tests in `tests/all/component_model/async.rs`.
Both run the same canonical-ABI sequence 1,000 times (`future.new`, then a
`future.write` that blocks, then `future.drop-readable`). The guest then waits
for the `FUTURE_WRITE`/`DROPPED` event with `waitable-set.wait` and calls
`future.drop-writable`. One test joins the writable end to the waitable set
before dropping the readable end, the other after. Both pass on `main`, and
the table never peaks above 100 entries. So Wasmtime handles the protocol
itself correctly.

### What happens

From `WASMTIME_LOG` traces of the wit-bindgen guest on `main`:

1. **Writer dropped:** `drop(writer)` makes wit-bindgen's `Drop for
   FutureWriter` start a deferred default-value write (`write_and_forget`).
   `future.write` returns `Blocked`, because nothing is reading yet. The
   writable handle joins the task's waitable set.
2. **Reader dropped:** `drop(reader)` calls `future.drop-readable`. Wasmtime
   correctly records a `FutureWrite { code: Dropped }` event for the writable
   handle.
3. **Export returns:** the guest's `run` future completes and calls
   `task.return`. But wit-bindgen won't exit a task while it still has
   registered waitables, so its callback returns `WAIT` instead.
4. **One event per turn:** Wasmtime queues a `DeliverEvent` work item for
   the task. Each such item gives the guest **one** event through its
   callback. The guest completes that deferred write and drops its writer,
   which frees the transmit state and both handles, then returns `WAIT`
   again. Wasmtime then queues the next `DeliverEvent` and suspends with
   `NeedWork`. So each future takes its own turn of the event loop.
5. **Nothing drives the loop:** `call_run` resolved at `task.return`, so
   `run_concurrent` returns without running any of those turns. Until the
   embedder drives the store's event loop again (once per future), the task,
   its thread, the waitable set, and 3 entries per future stay in the table.

### Why it looked like a real leak earlier

- **Inside the guest:** while the guest is busy creating more of these
  futures, the backlog only shrinks when the guest's task gets an event. On
  a yield, wit-bindgen appears to handle one ready event and then go back to
  the main future. That's from reading its `TaskState::callback`; I haven't
  measured it. That would explain why yielding or briefly waiting after each
  chunk of 1,000 didn't keep up.
- **A second `run` on the same instance** fails sooner because the instance
  is busy with the new task, so the first task's events can't be delivered.

My earlier conclusion that "the entries are never freed" was wrong. They're
freed once the guest task gets enough event-loop turns.

### Impact

- **Bounded only by the table limit:** any wit-bindgen guest that drops
  future writers while the reader is open but unread builds up 3 table
  entries per future. That includes keeping the reader, or handing it to
  something that doesn't read it promptly. If the guest stays busy or the
  host stops driving the store, nothing else limits the growth. Long-lived
  instances can then exhaust the table and trap. Idle stores keep the state
  until something drives them.
- **Each cleanup costs a full event-loop turn plus a guest callback**,
  even when cleanup does keep up.
- **Not hit by `p3_http_proxy`:** the completion future it hands to
  `consume-body` is read by the host immediately, so the deferred write
  completes at once.

### Possible fixes

- **wit-bindgen:**
  - drain all ready events with `waitable-set.poll` in the callback before
    returning `WAIT`;
  - on a yield, drain all ready events before resuming the main future;
  - avoid the deferred write entirely when the reader is known to be gone.
- **Wasmtime:**
  - keep delivering ready events to a callback task within the same turn,
    instead of one `DeliverEvent` work item per turn;
  - or offer embedders a way to run a store until its guest work is idle.
- **Guests, as a workaround:** drop the reader first, or keep the writer
  until the reader has been dropped. `p3_http_api_bench` does the latter.

## Reproducing

- **Benchmarks:**
  `cargo bench -p wasmtime-wasi-http --bench p3_http_proxy --features test-programs-artifacts/release-artifacts`,
  run on `abk/p3-http-api-benches` (base) and on `abk/opt3f-typed-table`.
- **Profile data:** in
  `/tmp/claude-1000/-home-adam-wasmtime/4d0010e7-cb9b-47aa-aea4-e8c120526c0c/scratchpad/`:
  - `perf2.data` and `bins2/opt3f-prof` (the binary it came from);
  - `analysis2.txt`, `analyze.py` and `callers.py`.
- **Step 5 reproduction:** in `/home/adam/wasmtime-leak`, on
  `abk/future-writer-leak`:
  - `cargo test -p wasmtime-wasi --test all p3_future_writer_drop` (fails on `main`)
  - `cargo test --test all -- future_write_` (passes)
