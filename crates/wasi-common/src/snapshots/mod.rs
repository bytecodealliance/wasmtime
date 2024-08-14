//! One goal of `wasi-common` is for multiple WASI snapshots to provide an
//! interface to the same underlying `crate::WasiCtx`. This provides us a path
//! to evolve WASI by allowing the same WASI Command to import functions from
//! different snapshots - e.g. the user could use Rust's `std` which imports
//! snapshot 1, but also depend directly on the `wasi` crate which imports
//! some future snapshot 2. Right now, this amounts to supporting snapshot 1
//! and "snapshot 0" aka wasi_unstable at once.
//!
//! The architectural rules for snapshots are:
//!
//! * Snapshots are arranged into modules under `crate::snapshots::`.
//! * Each snapshot should invoke `wiggle::from_witx!` with `ctx:
//!   crate::WasiCtx` in its module, and impl all of the required traits.
//! * Snapshots can be implemented in terms of other snapshots. For example,
//!   snapshot 0 is mostly implemented by calling the snapshot 1 implementation,
//!   and converting its own types back and forth with the snapshot 1 types. In a
//!   few cases, that is not feasible, so snapshot 0 carries its own
//!   implementations in terms of the `WasiFile` and `WasiSched` traits.
//! * Snapshots can be implemented in terms of the `Wasi*` traits given by
//!   `WasiCtx`. No further downcasting via the `as_any` escape hatch is
//!   permitted.

pub mod preview_0;
pub mod preview_1;
