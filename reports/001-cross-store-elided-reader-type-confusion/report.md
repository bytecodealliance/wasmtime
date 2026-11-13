# Host-created `StreamReader`/`FutureReader` from another store skips the payload check and panics the host

- **Audited:** branch `abk/elide-reader-typecheck-followup` at `ccf918427f`
  (commits `071755bc18`, `fde9df7e72`, `b6281c9b2c`, `ccf918427f` on top of
  `main` `8c11e2e0ed`)
- **Compared with:** `main` `8c11e2e0ed`
- **Date:** 2026-11-03
- **Platform:** Linux 7.0.0-38-generic, x86_64; rustc 1.99.1
- **Auditor:** Claude Opus 5.5 (`claude-opus-5-5`)
- **Feature:** component-model-async (on by default), default `Config`
- **Class:** type confusion leading to a host panic (DoS). This is a
  regression from an error on `main`. The trigger requires embedder misuse
  (a reader used with the wrong store).

## Summary

This branch adds `TransmitCheck::Elided`. A `StreamReader<T>` or
`FutureReader<T>` created by `new` sets a tag bit in its
`HostResourceIndex`, and lowering such a reader into a guest skips
`typecheck_lowered_transmit` entirely. That's sound only if the index still
resolves to the transmit `new` created, whose producer yields `T`.

The index isn't bound to a store. `HostResourceData::cur_generation` starts
at 0 in every store, so the first host stream in two fresh stores gets the
same `HostResourceIndex` (index 1, generation 0). When a tagged reader from
store A is lowered in store B:

1. `lower_stream_to_index` (`concurrent/futures_and_streams.rs`) runs
   `TransmitCheck::Elided.run`, which returns `Ok(())` without looking at
   the transmit;
2. `host_reader_take` validates the index against **B's** host table,
   where it matches B's own stream, and moves that stream into the guest;
3. when the guest reads, `lower::<P::Item>` writes B's producer's items
   (here `String`) using the guest's payload type (`u32`). The `Lower` impl
   assumes the two match, and the host panics.

On `main`, step 1 is `typecheck_lowered_transmit`, which looks up B's
transmit's real origin and returns "future or stream payload types differ".
The `TransmitCheck::Static` path on this branch also does that lookup, so
only `Elided` (readers fresh from `new`) is affected.

This is the same sink as finding 0157 (GHSA-9pvw-hwfg-92h5): items lowered
with a payload type that doesn't match `P::Item`. It's reached through a
different way of breaking the "reader type == producer item type"
invariant.

## Reproduction

`poc/` is a standalone crate that depends on this checkout via
`path = "../../../crates/wasmtime"`.

```sh
cd reports/001-cross-store-elided-reader-type-confusion/poc
CARGO_TARGET_DIR=../../../target/audit-poc cargo build --release
../../../target/audit-poc/release/cross-store-reader-poc mismatch
../../../target/audit-poc/release/cross-store-reader-poc match
```

What it does:

- Creates two `Store`s with the same component. Its export
  `consume-u: async func(s: stream<u32>) -> u32` reads one `u32` from `s`.
- Store A: `StreamReader::new(&mut store_a, vec![0xaaaa_u32])` (tagged).
- Store B:
  - `mismatch`: `StreamReader::new(&mut store_b, vec!["hello".to_string()])`;
  - `match` (control): `StreamReader::new(&mut store_b, vec![0xbbbb_u32])`.
- Calls B's `consume-u` with **A's** reader, through
  `get_typed_func::<(StreamReader<u32>,), (u32,)>`.

To test `main`, point the `wasmtime` path dependency at a `main` checkout.

### Results

| case | `main` `8c11e2e0ed`, release | branch `ccf918427f`, release | branch, debug |
|---|---|---|---|
| `mismatch` | `Err(future or stream payload types differ: expected string found u32)` | **panic:** `func/options.rs:167:69` `Option::unwrap()` on a `None` value | **panic:** `func/typed.rs:1361:9` `assertion failed: matches!(ty, InterfaceType::String)` |
| `match` | `Ok(0xbbbb)` | `Ok(0xbbbb)` | not run |

The debug backtrace shows the panic inside
`futures_and_streams::lower::<String, VecBuffer<String>, ()>`, called from
the host producer's `write` when the guest's `stream.read` is serviced.

The `match` control shows the underlying aliasing. It exists on `main`
too: A's reader silently resolves to B's stream, and the guest receives
B's `0xbbbb`. That part isn't new. What the branch changes is that a
type mismatch is no longer caught.

## Impact

- **Who can trigger it:** an embedder that passes a reader created in one
  `Store` to a call on another store, for example a multi-store proxy that
  forwards a body between per-request stores by mistake. Once the embedder
  makes that mistake, the guest only has to read the stream normally to
  trigger the panic. Which stream gets confused is determined by allocation
  order in the two stores.
- **Consequence:** a host panic in release builds (`realloc.unwrap()`), or
  `bad_type_info` panics for other type pairs, just as in 0157.
- **Memory safety:** guest memory writes in `lower()` are still
  bounds-checked against `T::SIZE32 * count`. I found no out-of-bounds
  host access, but I didn't explore beyond the panic.
- **API contract:** the reader docs say `close`, `pipe` and similar
  functions return an error "if it does not belong to `store`". On `main`,
  a type mismatch across stores is at least turned into an error by the
  origin check. The branch turns it into a panic.
- **Severity:** it needs embedder API misuse, so it's probably not a CVE
  under Wasmtime's policy. But it's a regression from `Err` to a host panic
  on a Tier 1, on-by-default feature, and should be fixed before the branch
  merges.

## Suggested fixes

Any one of these would work:

1. **Make indices store-specific.** Seed `HostResourceData::cur_generation`
   per store, e.g. from bits of the `StoreId`, so that an index from another
   store fails `validate_host_index`'s generation check. This costs
   nothing on the hot path, keeps `Elided`, and also fixes the pre-existing
   silent cross-store aliasing in the `match` case for every host resource.
   It isn't a guarantee, because generations can still collide.
2. **Keep `Elided` but validate the transmit cheaply.** After
   `host_reader_take`, compare the taken transmit's
   `TransmitOrigin::Host { item }` with `TypeId::of::<T>()` before inserting
   it into the guest. That's one more lookup, and it undoes part of the
   performance gain.
3. **Drop `Elided` and keep only `TransmitCheck::Static`.** It always looks
   up the transmit's real origin. With targeted inlining, it measured close
   to `Elided`.

## Related

- 0069 / GHSA-63h4-6cx9-pjfj and 0157 / GHSA-9pvw-hwfg-92h5 in
  `wasmtime-vuln-triage`: the same type-confusion sink. Their reproducers
  are still rejected on this branch (see `../findings/elide-reader-typecheck.md`).
