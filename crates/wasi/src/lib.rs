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

    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::Linker<T>,
        get_cx: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()>
    where
        U: Send
            + wasi_common::snapshots::preview_0::wasi_unstable::WasiUnstable
            + wasi_common::snapshots::preview_0::types::UserErrorConversion
            + wasi_common::snapshots::preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1
            + wasi_common::snapshots::preview_1::types::UserErrorConversion,
        T: Send,
    {
        snapshots::preview_1::add_to_linker(linker, get_cx)?;
        snapshots::preview_0::add_to_linker(linker, get_cx)?;
        Ok(())
    }

    pub mod snapshots {
        pub mod preview_1 {
            pub use wasi_common::snapshots::preview_1::wasi_snapshot_preview1::add_to_linker_blocking as add_to_linker;
        }
        pub mod preview_0 {
            pub use wasi_common::snapshots::preview_0::wasi_unstable::add_to_linker_blocking as add_to_linker;
        }
    }
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
    pub fn add_to_linker<T, U>(
        linker: &mut wasmtime::Linker<T>,
        get_cx: impl Fn(&mut T) -> &mut U + Send + Sync + Copy + 'static,
    ) -> anyhow::Result<()>
    where
        U: Send
            + wasi_common::snapshots::preview_0::wasi_unstable::WasiUnstable
            + wasi_common::snapshots::preview_0::types::UserErrorConversion
            + wasi_common::snapshots::preview_1::wasi_snapshot_preview1::WasiSnapshotPreview1
            + wasi_common::snapshots::preview_1::types::UserErrorConversion,
        T: Send,
    {
        snapshots::preview_1::add_to_linker(linker, get_cx)?;
        snapshots::preview_0::add_to_linker(linker, get_cx)?;
        Ok(())
    }

    pub mod snapshots {
        pub mod preview_1 {
            pub use wasi_common::snapshots::preview_1::wasi_snapshot_preview1::add_to_linker_async as add_to_linker;
        }
        pub mod preview_0 {
            pub use wasi_common::snapshots::preview_0::wasi_unstable::add_to_linker_async as add_to_linker;
        }
    }
}
