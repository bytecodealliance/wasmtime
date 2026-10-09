use wasmtime_core::{error::Result, format_err};

include!(concat!(env!("OUT_DIR"), "/gen.rs"));

/// Get the included gdbstub-adapter component artifact, or an error
/// if it is not included in this build.
pub fn gdbstub() -> Result<&'static [u8]> {
    crate::GDBSTUB_COMPONENT.ok_or_else(|| {
        format_err!(concat!(
            "gdbstub component not included in this build. Please build ",
            "in-tree with `--features gdbstub` or use release binaries.",
        ))
    })
}
