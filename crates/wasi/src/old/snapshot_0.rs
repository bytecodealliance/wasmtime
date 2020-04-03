use wasi_common::old::snapshot_0::hostcalls;
pub use wasi_common::old::snapshot_0::{WasiCtx, WasiCtxBuilder};

// Defines a `struct Wasi` with member fields and appropriate APIs for dealing
// with all the various WASI exports.
wig::define_wasi_struct!("phases/old/snapshot_0/witx/wasi_unstable.witx");

pub fn is_wasi_module(name: &str) -> bool {
    // FIXME: this should be more conservative, but while WASI is in flux and
    // we're figuring out how to support multiple revisions, this should do the
    // trick.
    name.starts_with("wasi")
}
