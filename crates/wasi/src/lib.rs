use wasmtime::Trap;

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
