use std::cell::RefCell;
use std::rc::Rc;
pub use wasi_common::{
    Error, FdFlags, FileCaps, Filestat, OFlags, ReaddirCursor, ReaddirEntity, SystemTimeSpec,
    WasiCtx, WasiCtxBuilder, WasiDir, WasiFile,
};
use wasmtime::{Linker, Store};

/// An instantiated instance of all available wasi exports. Presently includes
/// both the "preview1" snapshot and the "unstable" (preview0) snapshot.
pub struct Wasi {
    preview_1: snapshots::preview_1::Wasi,
    preview_0: snapshots::preview_0::Wasi,
}

impl Wasi {
    pub fn new(store: &Store, context: WasiCtx) -> Self {
        let context = Rc::new(RefCell::new(context));
        let preview_1 = snapshots::preview_1::Wasi::new(store, context.clone());
        let preview_0 = snapshots::preview_0::Wasi::new(store, context);
        Self {
            preview_1,
            preview_0,
        }
    }
    pub fn add_to_linker(&self, linker: &mut Linker) -> Result<(), anyhow::Error> {
        self.preview_1.add_to_linker(linker)?;
        self.preview_0.add_to_linker(linker)?;
        Ok(())
    }
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
                { name: Wasi,
                  docs: "An instantiated instance of the wasi exports.

This represents a wasi module which can be used to instantiate other wasm
modules. This structure exports all that various fields of the wasi instance
as fields which can be used to implement your own instantiation logic, if
necessary. Additionally [`Wasi::get_export`] can be used to do name-based
resolution.",
                },
            },
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
                { name: Wasi,
                  docs: "An instantiated instance of the wasi exports.

This represents a wasi module which can be used to instantiate other wasm
modules. This structure exports all that various fields of the wasi instance
as fields which can be used to implement your own instantiation logic, if
necessary. Additionally [`Wasi::get_export`] can be used to do name-based
resolution.",
                },
            },
        });
    }
}
