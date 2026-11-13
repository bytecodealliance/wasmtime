# Audit notes: elided reader typecheck branch

- **Audited:** `abk/elide-reader-typecheck-followup` at `ccf918427f`, vs.
  `main` `8c11e2e0ed`
- **Date:** 2026-11-03
- **Platform:** Linux 7.0.0-38-generic, x86_64; rustc 1.99.1
- **Auditor:** Claude Opus 5.5 (`claude-opus-5-5`)

Scope: the four commits on top of `main`. They add `TransmitCheck`
(`Elided`, `Static`, `Dynamic`), record `TypeId` in
`TransmitOrigin::Host`, store the "created by `new`" flag in bit 31 of the
readers' `HostResourceIndex`, and add a regression test.

## Confirmed

- **001: cross-store tagged reader.** `Elided` trusts the index to transmit
  binding, but indices aren't bound to a store. On the branch this causes a
  host panic; on `main` it's an error. See
  `../001-cross-store-elided-reader-type-confusion/report.md`.

## Checked, no bug found

### Regressions of 0069 and 0157

- All seven 0157 `relabel` probe cases (`sstr`, `ssb`, `sfo`, `mksb`, `val`,
  `direct`, `nested`) still return "future or stream payload types differ"
  on the branch, in a release build. The probe was rebuilt against
  `crates/wasmtime` in a scratch copy.
- **Relabeled readers are never tagged.** Lifting goes through
  `lift_from_index` → `new_`. Conversions from `*Any` go through
  `try_into_*_reader` → `new_`. Both are untagged, so they use `Static`,
  which looks up the real origin and runs the full check when the `TypeId`
  differs.
- **0069's host-to-host pipe path is unchanged.** It's `set_consumer` with
  the `UntypedWriteBuffer` runtime `TypeId` check.

### Tag constructors

- The tag is set only in `FutureReader::new` and `StreamReader::new`. Their
  bounds, `FutureProducer<_, Item = T>` and `StreamProducer<_, Item = T>`,
  make `T == P::Item`. `new_transmit` stores
  `typecheck_payload::<P::Item>` and `TypeId::of::<P::Item>()` from the
  same `P`.
- For an untouched `new` reader, the skipped dynamic check is the same
  function as the static `T::typecheck` that the `Lower` contract
  guarantees has passed. The destination types are the same: the static
  check uses the component's `InstanceType`, and the dynamic one uses
  `cx.instance_type()` of the same instance. Payloads containing `*Any` or
  `ResourceAny` are checked later, when each nested value is lowered as an
  item. That's the same behavior as on `main`.

### Tag bit handling

- **No overlap with the generation.** `TAG = 1 << 31` is in the low,
  table-index half. The generation is in the high 32 bits.
- **The bit is always free.** Host indices come only from
  `host_resource_lower_{own,borrow}` → `HandleTable::insert`, which bails
  when `next + 1 >= MAX_HANDLE` (`1 << 28`). A const assert ties
  `MAX_HANDLE` to `TAG`.
- **The tag never leaks.** `tagged_idx` is read only through `idx()`,
  which untags, and `tag()`. All table operations use `idx()`. A leaked
  tagged index would be out of bounds and return an error.

### Stale indices within one store

- Every path that frees a reader's host slot calls
  `validate_host_index(.., true)`, which bumps the store-wide
  `cur_generation`: `host_reader_take` (lowering, `close`, `set_consumer`)
  and `host_resource_drop`. Every insertion goes through
  `new_host_index`, which records the current generation. Generations only
  increase, so a stale tagged reader can't validate against a later
  occupant of its slot. Lowering takes `&self`, so lowering the same reader
  twice fails cleanly on the second `host_reader_take`.
- **Near miss:** `cur_generation` is a `u32`. In release builds it wraps
  after 2^32 removals; in debug builds that overflow panics. After a wrap,
  a stale tagged reader that the embedder kept and lowers again could
  resolve to a newer transmit and hit the same panic as 001. This ABA limit
  exists on `main` for every host resource (there, a type mismatch errors
  and a matching type silently aliases). It also needs embedder misuse and
  billions of operations, so it isn't reported separately. Fix 1 in report
  001 doesn't address it; fix 2 does.

### Skipped `concurrent_state_mut()` forcing

- `Elided` no longer calls `transmit_origin`, which called
  `concurrent_state_mut()` and so forced a deferred current thread. The
  rest of the lowering path, `host_reader_take` and
  `lower_transmit_to_index`, only touches the host and instance handle
  tables and `TransmitHandle`/`TransmitState`, and doesn't use the current
  thread. No behavior change was found.

### API changes

- `Lower for {Future,Stream}Reader<T>` now requires `T: 'static`.
  `new_transmit` requires `P::Item: 'static`. Constructing a reader already
  required this, so it doesn't affect safety.
