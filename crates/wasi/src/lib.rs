//! `wasmtime-wasi` now supports using multiple snapshots to interface to the
//! same `WasiCtx`!
//!
//! `wasmtime_wasi::Wasi::new(&Store, WasiCtx)` is a struct which owns your
//! `WasiCtx` and provides linkage to every available snapshot.
//!
//! Individual snapshots are available through
//! `wasmtime_wasi::snapshots::preview_{0, 1}::Wasi::new(&Store, Rc<RefCell<WasiCtx>>)`.

#[cfg(feature = "preview2")]
pub mod preview2;

pub use wasi_common::{Error, I32Exit, WasiCtx, WasiDir, WasiFile};

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
            + wasi_common::snapshots::preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1,
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
            errors: { errno => trappable Error },
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
            errors: { errno => trappable Error },
            $async_mode: *
        });
    }
}
}
}

/// Exit the process with a conventional OS error code as long as Wasmtime
/// understands the error. If the error is not an `I32Exit` or `Trap`, return
/// the error back to the caller for it to decide what to do.
///
/// Note: this function is designed for usage where it is acceptable for
/// Wasmtime failures to terminate the parent process, such as in the Wasmtime
/// CLI; this would not be suitable for use in multi-tenant embeddings.
#[cfg(feature = "exit")]
pub fn maybe_exit_on_error(e: anyhow::Error) -> anyhow::Error {
    use std::process;
    use wasmtime::Trap;

    // If a specific WASI error code was requested then that's
    // forwarded through to the process here without printing any
    // extra error information.
    if let Some(exit) = e.downcast_ref::<I32Exit>() {
        // Print the error message in the usual way.
        // On Windows, exit status 3 indicates an abort (see below),
        // so return 1 indicating a non-zero status to avoid ambiguity.
        if cfg!(windows) && exit.0 >= 3 {
            process::exit(1);
        }
        process::exit(exit.0);
    }

    // If the program exited because of a trap, return an error code
    // to the outside environment indicating a more severe problem
    // than a simple failure.
    if e.is::<Trap>() {
        eprintln!("Error: {:?}", e);

        if cfg!(unix) {
            // On Unix, return the error code of an abort.
            process::exit(128 + libc::SIGABRT);
        } else if cfg!(windows) {
            // On Windows, return 3.
            // https://docs.microsoft.com/en-us/cpp/c-runtime-library/reference/abort?view=vs-2019
            process::exit(3);
        }
    }

    e
}
