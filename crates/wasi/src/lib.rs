//! `wasmtime-wasi` now supports using multiple snapshots to interface to the
//! same `WasiCtx`!
//!
//! `wasmtime_wasi::Wasi::new(&Store, WasiCtx)` is a struct which owns your
//! `WasiCtx` and provides linkage to every available snapshot.
//!
//! Individual snapshots are available through
//! `wasmtime_wasi::snapshots::preview_{0, 1}::Wasi::new(&Store, Rc<RefCell<WasiCtx>>)`.

pub use wasi_common::{Error, WasiCtx, WasiDir, WasiFile};

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
    super::define_wasi!(async T: Send);
}

// The only difference between these definitions for sync vs async is whether
// the wasmtime::Funcs generated are async (& therefore need an async Store and an executor to run)
// or whether they have an internal "dummy executor" that expects the implementation of all
// the async funcs to poll to Ready immediately.
#[doc(hidden)]
#[macro_export]
macro_rules! define_wasi {
    ($async_mode:tt $($bounds:tt)*) => {

use wasmtime::Linker;

pub fn add_to_linker<T, U>(
    linker: &mut Linker<T>,
    get_cx: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
) -> anyhow::Result<()>
    where U: Send
            + wasi_common::snapshots::preview_0::wasi_unstable::WasiUnstable
            + wasi_common::snapshots::preview_0::types::UserErrorConversion
            + wasi_common::snapshots::preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1
            + wasi_common::snapshots::preview_1::types::UserErrorConversion,
        $($bounds)*
{
    snapshots::preview_1::add_wasi_snapshot_preview1_to_linker(linker, get_cx)?;
    snapshots::preview_0::add_wasi_unstable_to_linker(linker, get_cx)?;
    Ok(())
}

pub mod snapshots {
    pub mod preview_1 {
        wiggle::wasmtime_integration!({
            // The wiggle code to integrate with lives here:
            target: wasi_common::snapshots::preview_1,
            // This must be the same witx document as used above. This should be ensured by
            // the `WASI_ROOT` env variable, which is set in wasi-common's `build.rs`.
            witx: ["$WASI_ROOT/phases/snapshot/witx/wasi_snapshot_preview1.witx"],
            errors: { errno => Error },
            $async_mode: *
        });
    }
    pub mod preview_0 {
        wiggle::wasmtime_integration!({
            // The wiggle code to integrate with lives here:
            target: wasi_common::snapshots::preview_0,
            // This must be the same witx document as used above. This should be ensured by
            // the `WASI_ROOT` env variable, which is set in wasi-common's `build.rs`.
            witx: ["$WASI_ROOT/phases/old/snapshot_0/witx/wasi_unstable.witx"],
            errors: { errno => Error },
            $async_mode: *
        });
    }
}
}
}
