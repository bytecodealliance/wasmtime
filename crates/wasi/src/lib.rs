//! `wasmtime-wasi` now supports using multiple snapshots to interface to the
//! same `WasiCtx`!
//!
//! `wasmtime_wasi::Wasi::new(&Store, WasiCtx)` is a struct which owns your
//! `WasiCtx` and provides linkage to every available snapshot.
//!
//! Individual snapshots are available through
//! `wasmtime_wasi::snapshots::preview_{0, 1}::Wasi::new(&Store, Rc<RefCell<WasiCtx>>)`.

pub use wasi_common::{Error, WasiCtx, WasiCtxBuilder, WasiDir, WasiFile};

/// Re-export the commonly used wasi-cap-std-sync crate here. This saves
/// consumers of this library from having to keep additional dependencies
/// in sync.
#[cfg(feature = "sync")]
pub mod sync {
    pub use wasi_cap_std_sync::*;
    super::define_wasi!(block_on);
}

/// Sync mode is the "default" of this crate, so we also export it at the top
/// level.
#[cfg(feature = "sync")]
pub use sync::*;

/// Re-export the wasi-tokio crate here. This saves consumers of this library from having
/// to keep additional dependencies in sync.
#[cfg(feature = "tokio")]
pub mod tokio {
    pub use wasi_tokio::*;
    super::define_wasi!(async + Send);
}

// The only difference between these definitions for sync vs async is whether
// the wasmtime::Funcs generated are async (& therefore need an async Store and an executor to run)
// or whether they have an internal "dummy executor" that expects the implementation of all
// the async funcs to poll to Ready immediately.
#[doc(hidden)]
#[macro_export]
macro_rules! define_wasi {
    ($async_mode: tt $($bounds:tt)*) => {

use std::borrow::BorrowMut;
use wasmtime::Linker;

pub fn add_to_linker<T>(linker: &mut Linker<T>) -> anyhow::Result<()>
where
    T: BorrowMut<crate::WasiCtx> $($bounds)*,
{
    snapshots::preview_1::add_wasi_snapshot_preview1_to_linker(linker)?;
    snapshots::preview_0::add_wasi_unstable_to_linker(linker)?;
    Ok(())
}

pub mod snapshots {
    pub mod preview_1 {
        use wasi_common::WasiCtx;
        // Defines a `struct Wasi` with member fields and appropriate APIs for dealing
        // with all the various WASI exports.
        wasmtime_wiggle::wasmtime_integration!({
            // The wiggle code to integrate with lives here:
            target: wasi_common::snapshots::preview_1,
            // This must be the same witx document as used above. This should be ensured by
            // the `WASI_ROOT` env variable, which is set in wasi-common's `build.rs`.
            witx: ["$WASI_ROOT/phases/snapshot/witx/wasi_snapshot_preview1.witx"],
            // This must be the same ctx type as used for the target:
            ctx: WasiCtx,
            // This macro will emit a struct to represent the instance,
            // with this name and docs:
            modules: { wasi_snapshot_preview1 =>
                { name: wasi_snapshot_preview1,
                  docs: "An instantiated instance of the wasi exports.

This represents a wasi module which can be used to instantiate other wasm
modules. This structure exports all that various fields of the wasi instance
as fields which can be used to implement your own instantiation logic, if
necessary. Additionally [`Wasi::get_export`] can be used to do name-based
resolution.",
                },
            },
            $async_mode: *
        });
    }
    pub mod preview_0 {
        use wasi_common::WasiCtx;
        // Defines a `struct Wasi` with member fields and appropriate APIs for dealing
        // with all the various WASI exports.
        wasmtime_wiggle::wasmtime_integration!({
            // The wiggle code to integrate with lives here:
            target: wasi_common::snapshots::preview_0,
            // This must be the same witx document as used above. This should be ensured by
            // the `WASI_ROOT` env variable, which is set in wasi-common's `build.rs`.
            witx: ["$WASI_ROOT/phases/old/snapshot_0/witx/wasi_unstable.witx"],
            // This must be the same ctx type as used for the target:
            ctx: WasiCtx,
            // This macro will emit a struct to represent the instance,
            // with this name and docs:
            modules: { wasi_unstable =>
                { name: wasi_unstable,
                  docs: "An instantiated instance of the wasi exports.

This represents a wasi module which can be used to instantiate other wasm
modules. This structure exports all that various fields of the wasi instance
as fields which can be used to implement your own instantiation logic, if
necessary. Additionally [`Wasi::get_export`] can be used to do name-based
resolution.",
                },
            },
            $async_mode: *
        });
    }
}
}
}
