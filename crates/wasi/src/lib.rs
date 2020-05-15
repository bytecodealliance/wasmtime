use anyhow::Result;
use std::fs::File;
use wasmtime::{Linker, Store, Trap};

pub mod old;

pub use wasi_common::{WasiCtx, WasiCtxBuilder};

// Defines a `struct Wasi` with member fields and appropriate APIs for dealing
// with all the various WASI exports.
wig::define_wasi_struct_for_wiggle!("phases/snapshot/witx/wasi_snapshot_preview1.witx");

pub fn is_wasi_module(name: &str) -> bool {
    // FIXME: this should be more conservative, but while WASI is in flux and
    // we're figuring out how to support multiple revisions, this should do the
    // trick.
    name.starts_with("wasi")
}

/// Implement the WASI `proc_exit` function. This function is implemented here
/// instead of in wasi-common so that we can use the runtime to perform an
/// unwind rather than exiting the host process.
fn wasi_proc_exit(status: i32) -> Result<(), Trap> {
    // Check that the status is within WASI's range.
    if status >= 0 && status < 126 {
        Err(Trap::i32_exit(status))
    } else {
        Err(Trap::new(
            "exit with invalid exit status outside of [0..126)",
        ))
    }
}

/// Creates a new [`Linker`], similar to `Linker::new`, and initializes it
/// with WASI exports.
pub fn wasi_linker(
    store: &Store,
    preopen_dirs: &[(String, File)],
    argv: &[String],
    vars: &[(String, String)],
) -> Result<Linker> {
    let mut linker = Linker::new(store);

    // Add the current snapshot to the linker.
    let mut cx = WasiCtxBuilder::new();
    cx.inherit_stdio().args(argv).envs(vars);

    for (name, file) in preopen_dirs {
        cx.preopened_dir(file.try_clone()?, name);
    }

    let cx = cx.build()?;
    let wasi = Wasi::new(linker.store(), cx);
    wasi.add_to_linker(&mut linker)?;

    // Repeat the above, but this time for snapshot 0.
    let mut cx = old::snapshot_0::WasiCtxBuilder::new();
    cx.inherit_stdio().args(argv).envs(vars);

    for (name, file) in preopen_dirs {
        cx.preopened_dir(file.try_clone()?, name);
    }

    let cx = cx.build()?;
    let wasi = old::snapshot_0::Wasi::new(linker.store(), cx);
    wasi.add_to_linker(&mut linker)?;

    Ok(linker)
}
